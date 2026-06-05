//! 원장 분석 어댑터 — CSV/xlsx 임포트 위저드 + IDEA식 op + 멀티 데이터셋 스토어.
//! 인지 0, 결정론 로컬(§0). 다이얼로그는 Rust 측(DialogExt).

mod store;

use axon_core::domain::table::Table;
use serde::Serialize;
use std::sync::Mutex;
use store::{DatasetNode, DatasetStore};
use tauri::{AppHandle, Emitter, Manager, Runtime, State};
use tauri_plugin_dialog::DialogExt;

#[derive(Default)]
pub struct LedgerState {
    grid: Mutex<Option<Vec<Vec<String>>>>,
    store: Mutex<DatasetStore>,
}

#[derive(Serialize, Clone)]
pub struct Preview {
    rows: Vec<Vec<String>>,
    tail_rows: Vec<Vec<String>>,
    tail_start: usize,
    total: usize,
}

#[derive(Serialize, Clone)]
pub struct ImportResult {
    table: Table,
    dataset_id: String,
}

const PREVIEW_ROWS: usize = 20;

fn resolve_df<'a>(
    store: &'a DatasetStore,
    dataset_id: &Option<String>,
) -> Result<&'a axon_ledger::DataFrame, String> {
    match dataset_id {
        Some(id) => store
            .get(id)
            .map(|ds| &ds.df)
            .ok_or_else(|| format!("데이터셋 '{id}'을 찾을 수 없습니다.")),
        None => store
            .active_df()
            .ok_or_else(|| "먼저 파일을 가져오세요.".to_string()),
    }
}

pub fn open_and_import<R: Runtime>(app: &AppHandle<R>) {
    let handle = app.clone();
    app.dialog()
        .file()
        .add_filter("스프레드시트 (CSV·Excel)", &["csv", "xlsx"])
        .pick_file(move |picked| {
            let Some(fp) = picked else { return };
            let path = fp.to_string();
            let is_xlsx = path.to_lowercase().ends_with(".xlsx");
            let grid = std::fs::read(&path)
                .map_err(|e| e.to_string())
                .and_then(|bytes| axon_ledger::read_grid(bytes, is_xlsx));
            match grid {
                Ok(g) => {
                    let len = g.len();
                    let ts = len.saturating_sub(PREVIEW_ROWS);
                    let preview = Preview {
                        rows: g.iter().take(PREVIEW_ROWS).cloned().collect(),
                        tail_rows: g[ts..].to_vec(),
                        tail_start: ts,
                        total: len,
                    };
                    if let Ok(mut guard) = handle.state::<LedgerState>().grid.lock() {
                        *guard = Some(g);
                    }
                    let _ = handle.emit("ledger-preview", preview);
                }
                Err(e) => {
                    let _ = handle.emit("ledger-error", e);
                }
            }
        });
}

#[tauri::command]
pub fn ledger_open(app: AppHandle) {
    open_and_import(&app);
}

#[tauri::command]
pub fn ledger_import(
    header_row: usize,
    end_row: Option<usize>,
    columns: Option<Vec<usize>>,
    state: State<LedgerState>,
) -> Result<ImportResult, String> {
    let grid_guard = state.grid.lock().map_err(|_| "상태 잠금 실패".to_string())?;
    let grid = grid_guard.as_ref().ok_or_else(|| "먼저 파일을 여세요.".to_string())?;
    let df = axon_ledger::import_grid(grid, header_row, end_row, columns.as_deref())?;
    let table = axon_ledger::to_table(&df);
    let mut store = state.store.lock().map_err(|_| "상태 잠금 실패".to_string())?;
    let dataset_id = store.insert("임포트".to_string(), df, None);
    Ok(ImportResult { table, dataset_id })
}

#[tauri::command]
pub fn ledger_preview(state: State<LedgerState>) -> Result<Preview, String> {
    let guard = state.grid.lock().map_err(|_| "상태 잠금 실패".to_string())?;
    let grid = guard.as_ref().ok_or_else(|| "먼저 파일을 여세요.".to_string())?;
    let len = grid.len();
    let tail_start = len.saturating_sub(PREVIEW_ROWS);
    Ok(Preview {
        rows: grid.iter().take(PREVIEW_ROWS).cloned().collect(),
        tail_rows: grid[tail_start..].to_vec(),
        tail_start,
        total: len,
    })
}

#[tauri::command]
pub fn ledger_op(
    op: String,
    column: Option<String>,
    dataset_id: Option<String>,
    state: State<LedgerState>,
) -> Result<ImportResult, String> {
    let mut store = state.store.lock().map_err(|_| "상태 잠금 실패".to_string())?;
    let parent = dataset_id
        .clone()
        .or_else(|| store.active_id().map(String::from))
        .ok_or_else(|| "먼저 파일을 가져오세요.".to_string())?;
    let df = resolve_df(&store, &dataset_id)?;
    let (result_df, op_name) = match op.as_str() {
        "duplicates" => {
            let d = axon_ledger::find_duplicates(df).map_err(|e| e.to_string())?;
            (d, "중복검출")
        }
        "gaps" => {
            let col = column.as_deref().ok_or_else(|| "열을 선택하세요.".to_string())?;
            let gaps = axon_ledger::detect_gaps(df, col).map_err(|e| e.to_string())?;
            let table = Table {
                headers: vec!["누락 전표번호".to_string()],
                rows: gaps.iter().map(|n| vec![n.to_string()]).collect(),
                keys: Vec::new(),
            };
            let id = store.insert(format!("갭검출({col})"), axon_ledger::DataFrame::default(), Some(parent));
            return Ok(ImportResult { table, dataset_id: id });
        }
        "benford" => {
            let col = column.as_deref().ok_or_else(|| "열을 선택하세요.".to_string())?;
            let table = axon_ledger::benford(df, col).map_err(|e| e.to_string())?;
            let id = store.insert(format!("벤포드({col})"), axon_ledger::DataFrame::default(), Some(parent));
            return Ok(ImportResult { table, dataset_id: id });
        }
        _ => return Err(format!("알 수 없는 분석: {op}")),
    };
    let table = axon_ledger::to_table(&result_df);
    let id = store.insert(op_name.to_string(), result_df, Some(parent));
    Ok(ImportResult { table, dataset_id: id })
}

#[tauri::command]
pub fn ledger_counterpart(
    account_col: String,
    voucher_col: String,
    debit_col: String,
    credit_col: String,
    target: String,
    dataset_id: Option<String>,
    state: State<LedgerState>,
) -> Result<ImportResult, String> {
    let mut store = state.store.lock().map_err(|_| "상태 잠금 실패".to_string())?;
    let parent = dataset_id
        .clone()
        .or_else(|| store.active_id().map(String::from))
        .ok_or_else(|| "먼저 파일을 가져오세요.".to_string())?;
    let df = resolve_df(&store, &dataset_id)?;
    let cols = axon_ledger::CounterpartCols {
        account: &account_col,
        voucher: &voucher_col,
        debit: &debit_col,
        credit: &credit_col,
    };
    let out = axon_ledger::analyze_counterpart(df, &cols, &target, 1e-6).map_err(|e| e.to_string())?;
    let table = axon_ledger::to_table(&out);
    let id = store.insert(format!("상대전표({target})"), out, Some(parent));
    Ok(ImportResult { table, dataset_id: id })
}

#[tauri::command]
pub fn ledger_sample(
    amount_col: String,
    pm: f64,
    seed: u64,
    confidence: f64,
    dataset_id: Option<String>,
    state: State<LedgerState>,
) -> Result<ImportResult, String> {
    let mut store = state.store.lock().map_err(|_| "상태 잠금 실패".to_string())?;
    let parent = dataset_id
        .clone()
        .or_else(|| store.active_id().map(String::from))
        .ok_or_else(|| "먼저 파일을 가져오세요.".to_string())?;
    let df = resolve_df(&store, &dataset_id)?;
    let out =
        axon_ledger::monetary_unit_sampling(df, &amount_col, pm, seed, confidence)
            .map_err(|e| e.to_string())?;
    let table = axon_ledger::to_table(&out);
    let id = store.insert("MUS표본".to_string(), out, Some(parent));
    Ok(ImportResult { table, dataset_id: id })
}

// ─ 데이터셋 관리 커맨드 ─

#[tauri::command]
pub fn ledger_list_datasets(state: State<LedgerState>) -> Result<Vec<DatasetNode>, String> {
    let store = state.store.lock().map_err(|_| "상태 잠금 실패".to_string())?;
    Ok(store.list())
}

#[tauri::command]
pub fn ledger_get_dataset(id: String, state: State<LedgerState>) -> Result<Table, String> {
    let store = state.store.lock().map_err(|_| "상태 잠금 실패".to_string())?;
    let ds = store.get(&id).ok_or_else(|| format!("데이터셋 '{id}'을 찾을 수 없습니다."))?;
    Ok(axon_ledger::to_table(&ds.df))
}

#[tauri::command]
pub fn ledger_set_active(id: String, state: State<LedgerState>) -> Result<(), String> {
    let mut store = state.store.lock().map_err(|_| "상태 잠금 실패".to_string())?;
    if store.set_active(&id) {
        Ok(())
    } else {
        Err(format!("데이터셋 '{id}'을 찾을 수 없습니다."))
    }
}

#[tauri::command]
pub fn ledger_delete_dataset(id: String, state: State<LedgerState>) -> Result<(), String> {
    let mut store = state.store.lock().map_err(|_| "상태 잠금 실패".to_string())?;
    if store.remove(&id) {
        Ok(())
    } else {
        Err(format!("데이터셋 '{id}'을 찾을 수 없습니다."))
    }
}

#[tauri::command]
pub fn ledger_rename_dataset(
    id: String,
    name: String,
    state: State<LedgerState>,
) -> Result<(), String> {
    let mut store = state.store.lock().map_err(|_| "상태 잠금 실패".to_string())?;
    if store.rename(&id, name) {
        Ok(())
    } else {
        Err(format!("데이터셋 '{id}'을 찾을 수 없습니다."))
    }
}

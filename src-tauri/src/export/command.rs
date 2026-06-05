//! export_table 커맨드 — 형식별 Exporter 선택 + 저장 다이얼로그.

use super::pdf;
use super::xlsx::XlsxExporter;
use axon_core::domain::table::Table;
use axon_core::export::{csv::CsvExporter, json::JsonExporter};
use axon_core::port::exporter::Exporter;
use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;

fn export_bytes(
    format: &str,
    table: &Table,
    meta: &[(String, String)],
) -> Result<Vec<u8>, String> {
    match format {
        "pdf" => pdf::export_pdf(table, meta),
        "json" => Ok(JsonExporter.export(table)),
        "csv" => Ok(CsvExporter.export(table)),
        "xlsx" => Ok(XlsxExporter.export(table)),
        other => Err(format!("지원하지 않는 형식: {other}")),
    }
}

#[tauri::command]
pub async fn export_table(
    app: AppHandle,
    table: Table,
    format: String,
    meta: Option<Vec<(String, String)>>,
) -> Result<Option<String>, String> {
    let meta = meta.unwrap_or_default();
    let bytes = export_bytes(&format, &table, &meta)?;

    let label = meta
        .iter()
        .find(|(k, _)| k == "분석")
        .map(|(_, v)| v.as_str())
        .unwrap_or("결과");
    let today = chrono::Local::now().format("%Y%m%d");
    let filename = format!("{label}_{today}.{format}");

    let path = app
        .dialog()
        .file()
        .add_filter(&format, &[format.as_str()])
        .set_file_name(filename)
        .blocking_save_file();

    match path {
        Some(fp) => {
            let pb = fp.into_path().map_err(|e| e.to_string())?;
            std::fs::write(&pb, bytes).map_err(|e| e.to_string())?;
            Ok(Some(pb.display().to_string()))
        }
        None => Ok(None),
    }
}

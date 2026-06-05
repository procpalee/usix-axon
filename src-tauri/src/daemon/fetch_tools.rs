//! 데이터 조회 도구 — 데몬이 자연어로 지시한 회사·기간을 OpenDART 조회로 실행한다.
//! search_company = 회사명→코드 후보(데몬이 모호성 판단), load_financials = 조회 후 화면 로드 + 분석대상 설정.
//! 인지(어느 회사·언제)는 데몬, 실행(검색·조회·렌더)은 여기(결정론).

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use super::fin_context;
use crate::dart::{corp, opendart};
use axon_core::domain::table::Table;

/// 화면 갱신 payload — 프론트 listen("statement-loaded")가 메인 뷰에 렌더.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct StatementLoaded {
    table: Table,
    corp_code: String,
    corp_name: String,
    start: i32,
    end: i32,
    basis: String,
    reprt: String,
}

/// 회사명 부분일치 검색 → 코드 후보(데몬이 보고 고른다). 최대 20건 표기.
pub(super) async fn search_company(app: &AppHandle, args: &serde_json::Value) -> (String, bool) {
    let q = args
        .get("query")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("")
        .trim();
    if q.is_empty() {
        return ("query(회사명)가 비었습니다.".into(), false);
    }
    match corp::search_corp(app.clone(), q.to_string()).await {
        Ok(list) if list.is_empty() => (format!("'{q}' 검색 결과 없음."), true),
        Ok(list) => {
            let mut s = format!("회사 검색 '{q}' → {}건:", list.len());
            for c in list.iter().take(20) {
                let stock = if c.stock.trim().is_empty() {
                    "비상장"
                } else {
                    c.stock.trim()
                };
                s.push_str(&format!("\n{} · {} ({stock})", c.code, c.name));
            }
            (s, true)
        }
        Err(e) => (format!("회사 검색 실패: {e}"), false),
    }
}

/// 회사·기간 재무제표 조회 → 화면 로드 + 분석대상 설정 + 요약. 이후 compute_footing 이 이 대상을 검증.
pub(super) async fn load_financials(app: &AppHandle, args: &serde_json::Value) -> (String, bool) {
    let corp_code = args
        .get("corp_code")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("")
        .trim();
    if corp_code.is_empty() {
        return (
            "corp_code 가 필요합니다 (search_company 로 먼저 찾으세요).".into(),
            false,
        );
    }
    let (Some(start), Some(end)) = (
        args.get("start").and_then(serde_json::Value::as_i64),
        args.get("end").and_then(serde_json::Value::as_i64),
    ) else {
        return ("start·end(연도)가 필요합니다.".into(), false);
    };
    let (start, end) = (start as i32, end as i32);
    let basis = args
        .get("basis")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    let reprt = args
        .get("reprt")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    let corp_name = args
        .get("corp_name")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("")
        .to_string();

    let facts = match opendart::fetch_facts_range(
        corp_code,
        start,
        end,
        opendart::norm_basis(basis.clone()),
        opendart::norm_reprt(reprt.clone()),
    )
    .await
    {
        Ok(f) => f,
        Err(e) => return (format!("재무제표 조회 실패: {e}"), false),
    };
    if facts.is_empty() {
        return (
            format!("[{corp_code} {start}~{end}] 조회된 데이터가 없습니다."),
            true,
        );
    }

    let table = Table::pivot(&facts);
    let rows = table.rows.len();
    // 화면 갱신 — 데몬 조회 결과를 메인 뷰에 반영(사용자가 눈으로 확인 = 환각 방지).
    let _ = app.emit(
        "statement-loaded",
        StatementLoaded {
            table,
            corp_code: corp_code.to_string(),
            corp_name: corp_name.clone(),
            start,
            end,
            basis: basis.clone().unwrap_or_default(),
            reprt: reprt.clone().unwrap_or_default(),
        },
    );
    // 분석 대상 박기 — compute_footing 등 후속 도구가 이 대상을 검증.
    fin_context::set(corp_code.to_string(), start, end, basis, reprt);

    let who = if corp_name.is_empty() {
        corp_code.to_string()
    } else {
        format!("{corp_name}({corp_code})")
    };
    (
        format!("[{who} {start}~{end}] 재무제표 {rows}개 계정 로드 완료 — 화면에 표시했고 분석 대상으로 설정했습니다."),
        true,
    )
}

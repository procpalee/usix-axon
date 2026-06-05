//! 현재 분석 대상(회사·기간) 보관 — 데몬 도구가 "지금 화면 회사"를 결정론적으로 검증하게.
//!
//! 데몬은 회사코드를 args 로 넘기지 않는다(환각 시 엉뚱한 회사 검증 = 조용한 회계 오류).
//! 프론트가 재무제표를 조회할 때 set_fin_context 로 대상을 박고, compute_* 도구는 이 전역에서
//! 대상을 읽는다 → 화면에 로드된 바로 그 회사만 검증된다. 세션성이라 메모리(앱 종료 시 소멸).

use serde::Serialize;
use std::sync::Mutex;

/// 현재 분석 대상. basis/reprt 는 raw(미정규화) — 조회 시점에 opendart 가 정규화.
#[derive(Clone, Serialize)]
pub(crate) struct FinQuery {
    pub corp_code: String,
    pub start: i32,
    pub end: i32,
    pub basis: Option<String>,
    pub reprt: Option<String>,
}

static FIN: Mutex<Option<FinQuery>> = Mutex::new(None);

/// 현재 분석 대상 (미설정이면 None).
pub(crate) fn get() -> Option<FinQuery> {
    FIN.lock().ok().and_then(|g| g.clone())
}

/// 분석 대상 지정 — 프론트가 재무제표 조회에 성공하면 호출(화면 = 검증 대상 일치).
/// 내부 설정 — command(set_fin_context)와 도구(load_financials)가 공유.
pub(crate) fn set(corp_code: String, start: i32, end: i32, basis: Option<String>, reprt: Option<String>) {
    if let Ok(mut g) = FIN.lock() {
        *g = Some(FinQuery {
            corp_code,
            start,
            end,
            basis,
            reprt,
        });
    }
}

#[tauri::command]
pub fn set_fin_context(
    corp_code: String,
    start: i32,
    end: i32,
    basis: Option<String>,
    reprt: Option<String>,
) {
    set(corp_code, start, end, basis, reprt);
}

/// 분석 대상 해제 (대화 초기화·화면 비움 시).
#[tauri::command]
pub fn clear_fin_context() {
    if let Ok(mut g) = FIN.lock() {
        *g = None;
    }
}

/// 현재 분석 대상을 프론트에 노출 — 조서 창 헤더 등. 미설정이면 null.
#[tauri::command]
pub fn get_fin_context() -> Option<FinQuery> {
    get()
}

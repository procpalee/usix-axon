//! 감사조서 도구 — 데몬이 tool_execute 로 위임하는 결정론 산출물 생성.
//! 대상 = fin_context(화면에 로드된 회사·기간), 계산 = axon-core(LLM 0).
//! 데몬은 "현재 화면 합계검증" 만 지시 — 회사코드·수치를 wire 로 받지 않는다(환각·유출 차단).

use super::fin_context::{self, FinQuery};
use crate::dart::opendart;
use axon_core::domain::footing::FootingCheck;
use axon_core::service::footing::verify_footing;

/// 합계검증(회계등식·소계 일관성) — 현재 분석 대상을 재조회해 검증 결과를 텍스트로.
/// 데몬이 이 텍스트를 받아 조서 서술을 구성한다.
pub(super) async fn compute_footing() -> (String, bool) {
    let Some(q) = fin_context::get() else {
        return (
            "분석 대상이 없습니다 — 먼저 회사·기간으로 재무제표를 조회하세요.".into(),
            false,
        );
    };
    let facts = match opendart::fetch_facts_range(
        &q.corp_code,
        q.start,
        q.end,
        opendart::norm_basis(q.basis.clone()),
        opendart::norm_reprt(q.reprt.clone()),
    )
    .await
    {
        Ok(f) => f,
        Err(e) => return (format!("재무제표 조회 실패: {e}"), false),
    };
    (render_footing(&verify_footing(&facts), &q), true)
}

/// FootingCheck 목록 → 데몬이 읽을 평문. 기간·항목·일치여부(불일치는 차이 금액).
fn render_footing(checks: &[FootingCheck], q: &FinQuery) -> String {
    let mut s = format!("[합계검증 · {} · {}~{}]", q.corp_code, q.start, q.end);
    if checks.is_empty() {
        s.push_str("\n검증 가능한 항목이 없습니다 (필요 계정 누락).");
        return s;
    }
    for c in checks {
        let status = if c.ok {
            "일치".to_string()
        } else {
            format!("불일치 (차이 {})", c.display())
        };
        s.push_str(&format!("\n{} · {} → {status}", c.period, c.name));
    }
    let pass = checks.iter().filter(|c| c.ok).count();
    s.push_str(&format!("\n\n검증 {}건 중 {}건 일치.", checks.len(), pass));
    s
}

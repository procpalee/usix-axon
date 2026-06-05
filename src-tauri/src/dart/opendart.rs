//! OpenDART API 클라이언트 — 키체인 키로 재무제표 조회 → Table 변환.
//! (OpenDART 는 기본 reqwest User-Agent 를 거부하므로 UA 를 명시한다.)

use super::keychain;
use axon_core::domain::{fact::Fact, money::Money, statement::Statement, table::Table};
use axon_core::service::footing::verify_footing;
use axon_core::service::ratio::compute_ratios;
use serde::Deserialize;

const BASE: &str = "https://opendart.fss.or.kr/api";

#[derive(Deserialize)]
struct DartResponse {
    status: String,
    message: String,
    #[serde(default)]
    list: Vec<DartItem>,
}

#[derive(Deserialize)]
struct DartItem {
    /// 재무제표 구분 코드 (BS/IS/CIS/CF/SCE) — 분할·분류의 키.
    #[serde(default)]
    sj_div: String,
    /// XBRL 표준계정ID — 회사·연도 간 비율 분석의 키 (비표준이면 빈 값).
    #[serde(default)]
    account_id: String,
    #[serde(default)]
    account_nm: String,
    #[serde(default)]
    thstrm_amount: String,
    /// 당기누적금액 — 분기/반기 보고서의 누적값(유량 제표용).
    #[serde(default)]
    thstrm_add_amount: String,
    #[serde(default)]
    frmtrm_amount: String,
    #[serde(default)]
    bfefrmtrm_amount: String,
    // account_detail(SCE 2축)·ord(트리 정렬)·currency 는 해당 기능 붙을 때 수복.
}

/// basis 정규화 — "OFS"(별도재무제표)만 별도, 그 외·없음은 "CFS"(연결) 기본.
pub(crate) fn norm_basis(basis: Option<String>) -> &'static str {
    match basis.as_deref() {
        Some("OFS") => "OFS",
        _ => "CFS",
    }
}

/// 보고서 종류 정규화 — 11012(반기)·11013(1분기)·11014(3분기), 그 외·없음은 11011(사업보고서).
pub(crate) fn norm_reprt(reprt: Option<String>) -> &'static str {
    match reprt.as_deref() {
        Some("11012") => "11012",
        Some("11013") => "11013",
        Some("11014") => "11014",
        _ => "11011",
    }
}

/// 단일 연도 조회 → Fact(당기/전기/전전기를 long 으로 melt). 데이터 없음(013)은 빈 벡터로
/// (범위 조회에서 오래된 연도가 누락돼도 전체가 실패하지 않게).
pub(crate) async fn fetch_one(
    corp_code: &str,
    year: &str,
    basis: &str,
    reprt: &str,
) -> Result<Vec<Fact>, String> {
    let key = keychain::get_key().map_err(|_| "OpenDART 키가 설정되지 않았습니다.".to_string())?;

    let client = reqwest::Client::builder()
        .user_agent("axon/0.1")
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .get(format!("{BASE}/fnlttSinglAcntAll.json"))
        .query(&[
            ("crtfc_key", key.as_str()),
            ("corp_code", corp_code),
            ("bsns_year", year),
            ("reprt_code", reprt),
            ("fs_div", basis),
        ])
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if resp.status().is_redirection() {
        return Err("OpenDART가 요청을 거부했습니다 (키 활성화/등록 IP 확인).".to_string());
    }

    let body = resp.text().await.map_err(|e| e.to_string())?;
    let parsed: DartResponse =
        serde_json::from_str(&body).map_err(|e| format!("응답 파싱 실패: {e}"))?;

    if parsed.status != "000" {
        if parsed.status == "013" {
            return Ok(vec![]); // 조회된 데이터 없음 = 빈
        }
        let reason = match parsed.status.as_str() {
            "010" => "등록되지 않은 인증키입니다.",
            "011" => "사용할 수 없는 키입니다(OpenDART 이용 등록 필요).",
            "012" => "접근할 수 없는 IP입니다.",
            "020" => "요청 제한을 초과했습니다.",
            "021" => "조회 가능한 회사 개수를 초과했습니다.",
            "100" | "101" => "요청 값이 부적절합니다.",
            "800" => "OpenDART 시스템 점검 중입니다.",
            "901" => "사용자 계정이 만료되었습니다.",
            _ => {
                return Err(format!(
                    "OpenDART 오류 [{}]: {}",
                    parsed.status, parsed.message
                ));
            }
        };
        return Err(format!("[{}] {reason}", parsed.status));
    }

    let base_year: i32 = year.parse().unwrap_or(0);
    let cumulative = reprt != "11011"; // 분기/반기 = 유량 제표는 당기누적금액 우선
    Ok(parsed
        .list
        .iter()
        .flat_map(|d| match Statement::from_sj_div(&d.sj_div) {
            // 알 수 없는 구분(이 엔드포인트는 5종만 반환)은 건너뛴다.
            Some(stmt) => {
                let current = if cumulative && stmt.is_flow() && !d.thstrm_add_amount.is_empty() {
                    Money::parse(&d.thstrm_add_amount)
                } else {
                    Money::parse(&d.thstrm_amount)
                };
                Fact::melt(
                    stmt,
                    &d.account_id,
                    &d.account_nm,
                    base_year,
                    [
                        current,
                        Money::parse(&d.frmtrm_amount),
                        Money::parse(&d.bfefrmtrm_amount),
                    ],
                )
            }
            None => Vec::new(),
        })
        .collect())
}

/// 단일 연도(당기/전기/전전기 3개년). basis=CFS(연결)/OFS(별도), 미지정 시 CFS.
#[tauri::command]
pub async fn fetch_financials(
    corp_code: String,
    year: String,
    basis: Option<String>,
    reprt: Option<String>,
) -> Result<Table, String> {
    let facts = fetch_one(&corp_code, &year, norm_basis(basis), norm_reprt(reprt)).await?;
    Ok(Table::pivot(&facts))
}

/// 연도 범위 전체 — 3년씩 띄엄 동시조회 후 Fact 병합. 재무제표·비율 조회의 공통 소스.
pub(crate) async fn fetch_facts_range(
    corp_code: &str,
    start: i32,
    end: i32,
    basis: &str,
    reprt: &str,
) -> Result<Vec<Fact>, String> {
    if end < start {
        return Err("연도 범위가 잘못되었습니다.".to_string());
    }
    let mut bases = Vec::new();
    let mut y = end;
    while y >= start {
        bases.push(y);
        y -= 3;
    }
    let results = futures_util::future::join_all(bases.into_iter().map(|b| {
        let cc = corp_code.to_string();
        async move { fetch_one(&cc, &b.to_string(), basis, reprt).await }
    }))
    .await;
    let mut facts = Vec::new();
    for r in results {
        facts.extend(r?);
    }
    Ok(facts)
}

/// 연도 범위 재무제표 — 계정×연도 표. basis=CFS(연결)/OFS(별도), 미지정 시 CFS.
#[tauri::command]
pub async fn fetch_financials_range(
    corp_code: String,
    start: i32,
    end: i32,
    basis: Option<String>,
    reprt: Option<String>,
) -> Result<Table, String> {
    Ok(Table::pivot(
        &fetch_facts_range(&corp_code, start, end, norm_basis(basis), norm_reprt(reprt)).await?,
    ))
}

/// 연도 범위 재무비율 — Fact 조회 후 결정론 비율 계산 → 지표×연도 표.
#[tauri::command]
pub async fn fetch_ratios(
    corp_code: String,
    start: i32,
    end: i32,
    basis: Option<String>,
    reprt: Option<String>,
) -> Result<Table, String> {
    Ok(Table::from_ratios(&compute_ratios(
        &fetch_facts_range(&corp_code, start, end, norm_basis(basis), norm_reprt(reprt)).await?,
    )))
}

/// 연도 범위 합계검증 — Fact 조회 후 회계등식·소계 일관성 검증 → 항목×연도 표.
#[tauri::command]
pub async fn fetch_footing(
    corp_code: String,
    start: i32,
    end: i32,
    basis: Option<String>,
    reprt: Option<String>,
) -> Result<Table, String> {
    Ok(Table::from_footing(&verify_footing(
        &fetch_facts_range(&corp_code, start, end, norm_basis(basis), norm_reprt(reprt)).await?,
    )))
}

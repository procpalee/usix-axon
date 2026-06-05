//! xbrlTaxonomy.json fetch + 로컬 캐시(`app_data_dir/taxonomy/{sj_div}.json`).
//! **IFRS 저작권 → 캐시만, repo 커밋 금지.** corp.rs `ensure_cache` 패턴 동형(있으면 재사용).
//!
//! ⚠️ 응답 필드는 라이브로 확정 — 현재 account_id/account_nm/label_kor/label_eng/ord 캡처.
//! calc 링크(parent)가 응답에 없으면 평면 택사노미(라벨·표준집합·alias 역인덱스는 그대로 유효).
//! 캐시는 *원본 JSON* 으로 둔다 — 파서가 좋아지면 재패치 없이 반영된다.

use super::keychain;
use axon_core::domain::statement::Statement;
use axon_core::domain::taxonomy::{Taxonomy, TaxonomyConcept};
use serde::Deserialize;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

const BASE: &str = "https://opendart.fss.or.kr/api";

#[derive(Deserialize)]
struct TaxonomyResponse {
    status: String,
    #[serde(default)]
    message: String,
    #[serde(default)]
    list: Vec<TaxonomyItem>,
}

/// 택사노미 한 행 — 누락 필드에 견디게 lenient. (실응답: account_id/account_nm/label_kor/label_eng/ifrs_ref.)
#[derive(Deserialize)]
struct TaxonomyItem {
    #[serde(default)]
    account_id: String,
    #[serde(default)]
    account_nm: String,
    #[serde(default)]
    label_kor: String,
    #[serde(default)]
    label_eng: String,
    /// K-IFRS 근거 문단(감사 레퍼). 공백만 오기도 함 → trim.
    #[serde(default)]
    ifrs_ref: String,
}

/// Statement → 택사노미 대표 sj_div. 택사노미 sj_div 는 granular(BS1~4·IS1~4·CIS/DCIS·CF1~4·SCE1~2).
/// v1: 대표 1종만(BS1=유동/비유동 구분법 등). 변형 병합은 후속 개선.
fn repr_sj_div(stmt: Statement) -> &'static str {
    match stmt {
        Statement::Bs => "BS1",
        Statement::Is => "IS1",
        Statement::Cis => "CIS1",
        Statement::Cf => "CF1",
        Statement::Sce => "SCE1",
    }
}

/// 프론트 키(bs/is/cis/cf/sce) → Statement.
fn stmt_from_key(key: &str) -> Option<Statement> {
    match key {
        "bs" => Some(Statement::Bs),
        "is" => Some(Statement::Is),
        "cis" => Some(Statement::Cis),
        "cf" => Some(Statement::Cf),
        "sce" => Some(Statement::Sce),
        _ => None,
    }
}

/// {sj_div}.json 을 앱 데이터/taxonomy 에 캐시. 있으면 그 경로(택사노미는 거의 불변).
async fn ensure_cache(app: &AppHandle, sj_div: &str) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("taxonomy");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!("{sj_div}.json"));
    if path.exists() {
        return Ok(path);
    }

    let key = keychain::get_key().map_err(|_| "OpenDART 키가 설정되지 않았습니다.".to_string())?;
    let body = reqwest::Client::builder()
        .user_agent("axon/0.1")
        .build()
        .map_err(|e| e.to_string())?
        .get(format!("{BASE}/xbrlTaxonomy.json"))
        .query(&[("crtfc_key", key.as_str()), ("sj_div", sj_div)])
        .send()
        .await
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())?;

    // 상태 확인 후에만 캐시 — 에러 응답을 캐시하면 영구 오류로 굳는다.
    let parsed: TaxonomyResponse =
        serde_json::from_str(&body).map_err(|e| format!("응답 파싱 실패: {e}"))?;
    if parsed.status != "000" {
        return Err(format!("[{}] {}", parsed.status, parsed.message));
    }
    std::fs::write(&path, &body).map_err(|e| e.to_string())?;
    Ok(path)
}

/// 응답 행들 → 도메인 Taxonomy. concept_id 빈 행은 버린다(표준계정만).
fn to_taxonomy(stmt: Statement, items: &[TaxonomyItem]) -> Taxonomy {
    let concepts = items
        .iter()
        .enumerate()
        .filter(|(_, it)| !it.account_id.is_empty())
        .map(|(i, it)| {
            // 표준 한글 라벨 우선, 없으면 account_nm 폴백.
            let raw = if it.label_kor.is_empty() {
                &it.account_nm
            } else {
                &it.label_kor
            };
            TaxonomyConcept {
                concept_id: canon_id(&it.account_id),
                label_ko: strip_abstract(raw),
                label_en: strip_abstract(&it.label_eng),
                ifrs_ref: it.ifrs_ref.trim().to_string(),
                is_abstract: it.account_id.ends_with("Abstract"),
                parent: None, // xbrlTaxonomy 엔 calc 링크 없음 → 평면. 트리는 제표 그룹/순서로.
                ord: i as i32, // 응답에 ord 없음 → 등장 순서(=presentation 순서).
            }
        })
        .collect();
    Taxonomy {
        statement: stmt,
        concepts,
    }
}

/// 택사노미 concept_id(`ifrs_*`) → 재무제표(fnlttSinglAcntAll) 형식(`ifrs-full_*`). dart_·기타는 그대로.
/// 두 엔드포인트의 prefix 가 달라(`ifrs_CurrentAssets` vs `ifrs-full_CurrentAssets`) 안 맞추면 매칭이 깨진다.
fn canon_id(id: &str) -> String {
    match id.strip_prefix("ifrs_") {
        Some(rest) => format!("ifrs-full_{rest}"),
        None => id.to_string(),
    }
}

/// 라벨에서 " [abstract]" 꼬리표 제거(그룹 헤더의 표준 라벨을 깔끔히).
fn strip_abstract(label: &str) -> String {
    label.replace("[abstract]", "").trim().to_string()
}

/// 표준계정체계 조회 — statement 키(bs/is/cis/cf/sce) → 캐시/패치 → Taxonomy.
#[tauri::command]
pub async fn load_taxonomy(app: AppHandle, statement: String) -> Result<Taxonomy, String> {
    let stmt = stmt_from_key(&statement).ok_or_else(|| format!("알 수 없는 재무제표: {statement}"))?;
    let path = ensure_cache(&app, repr_sj_div(stmt)).await?;
    let body = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let parsed: TaxonomyResponse =
        serde_json::from_str(&body).map_err(|e| format!("캐시 파싱 실패: {e}"))?;
    Ok(to_taxonomy(stmt, &parsed.list))
}

/// 표준계정체계 섹션맵 — concept_id → 섹션 라벨(BS=자산/부채/자본). 트리 섹션 레이어용.
/// CF/SCE 처럼 섹션이 없는 제표는 빈 맵(계정이 제표 직속).
#[tauri::command]
pub async fn fetch_section_map(
    app: AppHandle,
    statement: String,
) -> Result<std::collections::HashMap<String, String>, String> {
    Ok(load_taxonomy(app, statement).await?.section_map())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_real_schema_to_taxonomy() {
        // 실제 xbrlTaxonomy.json 형태: ifrs_ prefix·[abstract] 꼬리표·공백 ifrs_ref·빈 concept_id.
        let json = r#"{"status":"000","message":"정상","list":[
            {"account_id":"ifrs_AssetsAbstract","account_nm":"AssetsAbstract","label_kor":"자산 [abstract]","label_eng":"Assets [abstract]","ifrs_ref":" "},
            {"account_id":"ifrs_CurrentAssets","label_kor":"유동자산","label_eng":"Current assets","ifrs_ref":"K-IFRS 1001 문단 60"},
            {"account_id":"dart_ShortTermOtherReceivables","label_kor":"미수금"},
            {"account_nm":"빈ID는버려짐"}
        ]}"#;
        let parsed: TaxonomyResponse = serde_json::from_str(json).unwrap();
        let tax = to_taxonomy(Statement::Bs, &parsed.list);
        assert_eq!(tax.concepts.len(), 3); // 빈 concept_id 행 제외
        // concept_id 정규화: ifrs_ → ifrs-full_, dart_ 는 그대로.
        assert_eq!(tax.concepts[0].concept_id, "ifrs-full_AssetsAbstract");
        assert_eq!(tax.concepts[2].concept_id, "dart_ShortTermOtherReceivables");
        // abstract 판정 + 라벨 [abstract] 꼬리표 제거.
        assert!(tax.concepts[0].is_abstract);
        assert_eq!(tax.concepts[0].label_ko, "자산");
        assert!(!tax.concepts[1].is_abstract);
        // ifrs_ref: 공백만이면 빈 문자열, 실제 근거는 보존.
        assert_eq!(tax.concepts[0].ifrs_ref, "");
        assert_eq!(tax.concepts[1].ifrs_ref, "K-IFRS 1001 문단 60");
        // 재무제표 형식(ifrs-full_)으로 라벨 조회됨.
        assert_eq!(tax.label_ko("ifrs-full_CurrentAssets"), Some("유동자산"));
    }

    #[test]
    fn maps_statement_keys_and_sj_div() {
        assert_eq!(stmt_from_key("cf"), Some(Statement::Cf));
        assert_eq!(stmt_from_key("zz"), None);
        assert_eq!(repr_sj_div(Statement::Bs), "BS1");
        assert_eq!(repr_sj_div(Statement::Sce), "SCE1");
    }
}

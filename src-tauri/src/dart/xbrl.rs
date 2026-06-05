//! XBRL calculation linkbase(`cal.xml`) 파싱 → CalcTree. (rcept_no 조회·ZIP 다운로드는 후속 단계.)
//!
//! cal.xml 구조: `<link:loc label→concept href>` + `<link:calculationArc from→to order weight>`.
//! loc 로 label→concept_id 사전을 만들고 arc 의 label 을 concept_id 로 치환해 트리를 세운다.
//! corp.rs 의 quick-xml 파싱 패턴과 동형. 회사 공시 XBRL(공공) — IFRS 원본 패키지 아님(저작권 무관).

use super::keychain;
use axon_core::domain::calc_tree::{CalcTree, FootingViolation};
use axon_core::domain::table::Table;
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use serde::Deserialize;
use std::collections::HashMap;
use std::io::{Cursor, Read};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

const BASE: &str = "https://opendart.fss.or.kr/api";

/// cal.xml 문자열 → CalcTree. self-closing(Empty)·일반(Start) 양쪽 처리.
pub fn parse_calc_linkbase(xml: &str) -> CalcTree {
    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();
    let mut loc: HashMap<String, String> = HashMap::new(); // xlink:label → concept_id
    let mut arcs: Vec<(String, String, f64, i32)> = Vec::new(); // (from_label, to_label, order, weight)

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Empty(e)) | Ok(Event::Start(e)) => match e.name().as_ref() {
                b"link:loc" => {
                    let (mut label, mut concept) = (String::new(), String::new());
                    for a in e.attributes().flatten() {
                        let v = String::from_utf8_lossy(&a.value);
                        match a.key.as_ref() {
                            b"xlink:label" => label = v.into_owned(),
                            // href = "...xsd#ifrs-full_Assets" → '#' 뒤가 concept_id.
                            b"xlink:href" => concept = v.rsplit('#').next().unwrap_or("").to_string(),
                            _ => {}
                        }
                    }
                    if !label.is_empty() && !concept.is_empty() {
                        loc.insert(label, concept);
                    }
                }
                b"link:calculationArc" => {
                    let (mut from, mut to) = (String::new(), String::new());
                    let (mut order, mut weight) = (0.0_f64, 1_i32);
                    for a in e.attributes().flatten() {
                        let v = String::from_utf8_lossy(&a.value);
                        match a.key.as_ref() {
                            b"xlink:from" => from = v.into_owned(),
                            b"xlink:to" => to = v.into_owned(),
                            b"order" => order = v.parse().unwrap_or(0.0),
                            b"weight" => weight = v.parse().unwrap_or(1),
                            _ => {}
                        }
                    }
                    if !from.is_empty() && !to.is_empty() {
                        arcs.push((from, to, order, weight));
                    }
                }
                _ => {}
            },
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    // arc 의 label → concept_id 치환. loc 에 없는 label 은 버린다.
    let resolved = arcs
        .into_iter()
        .filter_map(|(f, t, o, w)| Some((loc.get(&f)?.clone(), loc.get(&t)?.clone(), o, w)))
        .collect();
    CalcTree::from_arcs(resolved)
}

#[derive(Deserialize)]
struct ListResponse {
    status: String,
    #[serde(default)]
    message: String,
    #[serde(default)]
    list: Vec<ListItem>,
}

#[derive(Deserialize)]
struct ListItem {
    #[serde(default)]
    rcept_no: String,
}

/// corp_code + 사업연도 → 사업보고서 접수번호. (사업보고서는 다음해 상반기 제출 → 그 구간 검색.)
async fn find_rcept_no(corp_code: &str, year: i32) -> Result<String, String> {
    let key = keychain::get_key().map_err(|_| "OpenDART 키가 설정되지 않았습니다.".to_string())?;
    let bgn = format!("{}0101", year + 1);
    let end = format!("{}0630", year + 1);
    let body = reqwest::Client::builder()
        .user_agent("axon/0.1")
        .build()
        .map_err(|e| e.to_string())?
        .get(format!("{BASE}/list.json"))
        .query(&[
            ("crtfc_key", key.as_str()),
            ("corp_code", corp_code),
            ("bgn_de", bgn.as_str()),
            ("end_de", end.as_str()),
            ("pblntf_detail_ty", "A001"), // 정기공시 > 사업보고서
            ("page_count", "10"),
        ])
        .send()
        .await
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())?;
    let resp: ListResponse =
        serde_json::from_str(&body).map_err(|e| format!("공시 응답 파싱 실패: {e}"))?;
    if resp.status != "000" {
        return Err(format!("공시 조회 실패 [{}] {}", resp.status, resp.message));
    }
    resp.list
        .into_iter()
        .map(|i| i.rcept_no)
        .find(|r| !r.is_empty())
        .ok_or_else(|| format!("{year} 사업보고서를 찾지 못했습니다."))
}

/// fnlttXbrl.xml(ZIP) 받아 `_cal.xml` 만 앱 데이터에 캐시. 이미 있으면 그 경로.
/// ⚠️ 캐시는 회사 공시 XBRL(공공) — repo 커밋 금지(app_data_dir 는 repo 밖).
async fn ensure_cal_cache(app: &AppHandle, rcept_no: &str) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("xbrl");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!("{rcept_no}_cal.xml"));
    if path.exists() {
        return Ok(path);
    }

    let key = keychain::get_key().map_err(|_| "OpenDART 키가 설정되지 않았습니다.".to_string())?;
    let bytes = reqwest::Client::builder()
        .user_agent("axon/0.1")
        .build()
        .map_err(|e| e.to_string())?
        .get(format!("{BASE}/fnlttXbrl.xml"))
        .query(&[
            ("crtfc_key", key.as_str()),
            ("rcept_no", rcept_no),
            ("reprt_code", "11011"),
        ])
        .send()
        .await
        .map_err(|e| e.to_string())?
        .bytes()
        .await
        .map_err(|e| e.to_string())?;

    let mut archive =
        zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e| format!("XBRL ZIP 해제 실패: {e}"))?;
    let mut cal_xml = String::new();
    let mut found = false;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        if entry.name().ends_with("_cal.xml") {
            entry.read_to_string(&mut cal_xml).map_err(|e| e.to_string())?;
            found = true;
            break;
        }
    }
    if !found {
        return Err("XBRL 에 calculation linkbase(_cal.xml)가 없습니다.".to_string());
    }
    std::fs::write(&path, &cal_xml).map_err(|e| e.to_string())?;
    Ok(path)
}

/// 상장사 재무제표 계층 트리 — 해당 사업연도 사업보고서 XBRL 의 calculation linkbase.
/// (비상장·임포트는 후속: 여러 상장사 트리에서 표준 계층 사전을 뽑아 재사용.)
#[tauri::command]
pub async fn load_calc_tree(app: AppHandle, corp_code: String, year: i32) -> Result<CalcTree, String> {
    let rcept_no = find_rcept_no(&corp_code, year).await?;
    let path = ensure_cal_cache(&app, &rcept_no).await?;
    let xml = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    Ok(parse_calc_linkbase(&xml))
}

/// 여러 상장사 CalcTree → 표준 트리(다수결). 비상장·임포트 계층 fallback 용(프론트가 누적 풀을 보냄).
#[tauri::command]
pub fn consensus_tree(trees: Vec<CalcTree>) -> CalcTree {
    CalcTree::consensus(&trees)
}

/// footing 위반 목록 → 표. 위반 없으면 통과 메시지 한 줄.
fn violations_table(viol: &[FootingViolation]) -> Table {
    if viol.is_empty() {
        return Table {
            headers: vec!["합계검증".to_string()],
            rows: vec![vec!["✅ 모든 계층 합계가 일치합니다 (footing 통과)".to_string()]],
            keys: Vec::new(),
        };
    }
    Table {
        headers: vec![
            "계정ID".to_string(),
            "신고값".to_string(),
            "자식 합계".to_string(),
            "차이".to_string(),
        ],
        rows: viol
            .iter()
            .map(|v| {
                vec![
                    v.concept_id.clone(),
                    v.actual.to_string(),
                    v.expected.to_string(),
                    v.diff.to_string(),
                ]
            })
            .collect(),
        keys: Vec::new(),
    }
}

/// XBRL 정밀 합계검증 — 상장사 calculation linkbase 의 모든 parent = Σ(weight·child) 를
/// 해당 연도 실제 값과 대조. 신고값과 자식 합계가 어긋난 계정을 잡는다(상장사 한정 — cal 필요).
#[tauri::command]
pub async fn fetch_calc_footing(
    app: AppHandle,
    corp_code: String,
    year: i32,
    basis: String,
) -> Result<Table, String> {
    let tree = load_calc_tree(app, corp_code.clone(), year).await?;
    // cal 은 사업보고서(11011) 기준 → facts 도 11011 로 맞춰야 계산식이 일치.
    let facts = super::opendart::fetch_one(&corp_code, &year.to_string(), &basis, "11011").await?;
    let mut values = HashMap::new(); // <String, Decimal> 추론 — Money.0(rust_decimal)
    for f in facts {
        if f.period == year {
            values.insert(f.concept_id, f.value.0);
        }
    }
    Ok(violations_table(&tree.footing_violations(&values)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_cal_linkbase_to_tree() {
        // 실제 cal.xml 축약(self-closing loc/arc, link: prefix).
        let xml = r#"<link:calculationLink xlink:role="role-D210000">
            <link:loc xlink:label="L_Assets" xlink:href="x.xsd#ifrs-full_Assets" />
            <link:loc xlink:label="L_CA" xlink:href="x.xsd#ifrs-full_CurrentAssets" />
            <link:loc xlink:label="L_Cash" xlink:href="x.xsd#ifrs-full_CashAndCashEquivalents" />
            <link:loc xlink:label="L_NCA" xlink:href="x.xsd#ifrs-full_NoncurrentAssets" />
            <link:calculationArc xlink:from="L_Assets" xlink:to="L_NCA" order="2" weight="1" />
            <link:calculationArc xlink:from="L_Assets" xlink:to="L_CA" order="1" weight="1" />
            <link:calculationArc xlink:from="L_CA" xlink:to="L_Cash" order="1" weight="1" />
        </link:calculationLink>"#;
        let t = parse_calc_linkbase(xml);
        let kids = t.children_of("ifrs-full_Assets");
        assert_eq!(kids.len(), 2);
        assert_eq!(kids[0].concept_id, "ifrs-full_CurrentAssets"); // order 1 먼저
        assert_eq!(kids[1].concept_id, "ifrs-full_NoncurrentAssets");
        assert_eq!(
            t.children_of("ifrs-full_CurrentAssets")[0].concept_id,
            "ifrs-full_CashAndCashEquivalents"
        );
        assert_eq!(t.roots(), vec!["ifrs-full_Assets"]);
    }

    #[test]
    fn drops_arcs_with_unknown_locator() {
        // loc 에 없는 label 을 가리키는 arc 는 버려진다(깨진 참조 무시).
        let xml = r#"<root>
            <link:loc xlink:label="L_A" xlink:href="x#ifrs-full_Assets" />
            <link:calculationArc xlink:from="L_A" xlink:to="L_MISSING" order="1" weight="1" />
        </root>"#;
        assert!(parse_calc_linkbase(xml).is_empty());
    }
}

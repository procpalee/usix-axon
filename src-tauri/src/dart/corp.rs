//! 회사 검색 — OpenDART corpCode.xml(전체 회사 목록 ZIP)을 캐시하고 회사명으로 corp_code 조회.

use super::keychain;
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use serde::Serialize;
use std::io::Read;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

const BASE: &str = "https://opendart.fss.or.kr/api";

#[derive(Clone, Serialize)]
pub struct Corp {
    pub code: String,
    pub name: String,
    pub stock: String,
}

/// corpCode.xml(ZIP) 을 받아 CORPCODE.xml 을 앱 데이터에 캐시. 이미 있으면 그 경로.
async fn ensure_cache(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let xml_path = dir.join("CORPCODE.xml");
    if xml_path.exists() {
        return Ok(xml_path);
    }

    let key = keychain::get_key().map_err(|_| "OpenDART 키가 설정되지 않았습니다.".to_string())?;
    let bytes = reqwest::Client::builder()
        .user_agent("axon/0.1")
        .build()
        .map_err(|e| e.to_string())?
        .get(format!("{BASE}/corpCode.xml"))
        .query(&[("crtfc_key", key.as_str())])
        .send()
        .await
        .map_err(|e| e.to_string())?
        .bytes()
        .await
        .map_err(|e| e.to_string())?;

    let mut archive =
        zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e| e.to_string())?;
    let mut entry = archive.by_index(0).map_err(|e| e.to_string())?;
    let mut xml = String::new();
    entry.read_to_string(&mut xml).map_err(|e| e.to_string())?;
    std::fs::write(&xml_path, &xml).map_err(|e| e.to_string())?;
    Ok(xml_path)
}

/// 회사명 부분일치로 검색 (최대 50건).
#[tauri::command]
pub async fn search_corp(app: AppHandle, query: String) -> Result<Vec<Corp>, String> {
    let q = query.trim();
    if q.is_empty() {
        return Ok(vec![]);
    }
    let xml_path = ensure_cache(&app).await?;
    let xml = std::fs::read_to_string(&xml_path).map_err(|e| e.to_string())?;

    let matched = parse_corps(&xml)
        .into_iter()
        .filter(|c| c.name.contains(q))
        .take(50)
        .collect();
    Ok(matched)
}

/// CORPCODE.xml 의 <list> 반복을 Corp 목록으로. (corp_code/corp_name/stock_code)
fn parse_corps(xml: &str) -> Vec<Corp> {
    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();
    let mut corps = Vec::new();
    let (mut code, mut name, mut stock) = (String::new(), String::new(), String::new());
    let mut field = "";

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                field = match e.name().as_ref() {
                    b"corp_code" => "code",
                    b"corp_name" => "name",
                    b"stock_code" => "stock",
                    b"list" => {
                        code.clear();
                        name.clear();
                        stock.clear();
                        ""
                    }
                    _ => "",
                };
            }
            Ok(Event::Text(t)) => {
                let txt = String::from_utf8_lossy(&t).into_owned();
                match field {
                    "code" => code = txt,
                    "name" => name = txt,
                    "stock" => stock = txt,
                    _ => {}
                }
            }
            Ok(Event::End(e)) => {
                if e.name().as_ref() == b"list" && !code.is_empty() {
                    corps.push(Corp {
                        code: code.clone(),
                        name: name.clone(),
                        stock: stock.clone(),
                    });
                }
                field = "";
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    corps
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_list_entries() {
        let xml = r#"<result>
            <list><corp_code>00126380</corp_code><corp_name>삼성전자</corp_name><stock_code>005930</stock_code></list>
            <list><corp_code>00164779</corp_code><corp_name>SK하이닉스</corp_name><stock_code>000660</stock_code></list>
        </result>"#;
        let corps = parse_corps(xml);
        assert_eq!(corps.len(), 2);
        assert_eq!(corps[0].name, "삼성전자");
        assert_eq!(corps[0].code, "00126380");
        assert_eq!(corps[1].stock, "000660");
    }
}

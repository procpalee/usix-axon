//! 워크스페이스 한정 파일 도구. 모든 경로는 `policy::jail` 통과 필수.
//!
//!   read_file  { path }
//!   write_file { path, content }
//!   edit_file  { path, old_string, new_string }

use std::path::Path;

use super::policy::jail;

pub fn read_file(ws: &Path, args: &serde_json::Value) -> (String, bool) {
    let target = match jail(ws, args["path"].as_str().unwrap_or("")) {
        Ok(t) => t,
        Err(e) => return (e, false),
    };
    match std::fs::read_to_string(&target) {
        Ok(s) => (s, true),
        Err(e) => (format!("읽기 실패: {e}"), false),
    }
}

pub fn write_file(ws: &Path, args: &serde_json::Value) -> (String, bool) {
    let content = args["content"].as_str().unwrap_or("");
    let target = match jail(ws, args["path"].as_str().unwrap_or("")) {
        Ok(t) => t,
        Err(e) => return (e, false),
    };
    match std::fs::write(&target, content) {
        Ok(()) => (format!("{}바이트 기록 완료", content.len()), true),
        Err(e) => (format!("쓰기 실패: {e}"), false),
    }
}

pub fn edit_file(ws: &Path, args: &serde_json::Value) -> (String, bool) {
    let old = args["old_string"].as_str().unwrap_or("");
    let new = args["new_string"].as_str().unwrap_or("");
    if old.is_empty() {
        return ("old_string 이 비어있습니다.".into(), false);
    }
    let target = match jail(ws, args["path"].as_str().unwrap_or("")) {
        Ok(t) => t,
        Err(e) => return (e, false),
    };
    let body = match std::fs::read_to_string(&target) {
        Ok(s) => s,
        Err(e) => return (format!("읽기 실패: {e}"), false),
    };
    // 데몬 file_ops 와 동일 규약: 0건=미발견, 2건↑=모호 거부, 1건만 치환.
    match body.matches(old).count() {
        0 => ("old_string 을 찾지 못했습니다.".into(), false),
        1 => match std::fs::write(&target, body.replacen(old, new, 1)) {
            Ok(()) => ("편집 완료".into(), true),
            Err(e) => (format!("쓰기 실패: {e}"), false),
        },
        n => (format!("old_string 이 {n}곳에 중복 — 고유해야 합니다."), false),
    }
}

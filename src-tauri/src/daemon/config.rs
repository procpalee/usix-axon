//! 데몬 접속 설정 — base URL + Bearer 토큰을 OS 키체인에 보관한다.
//!
//! 토큰 값은 WebView 로 안 내려가고 존재 여부(`has_daemon_token`)만 노출한다.

use keyring::Entry;

const SERVICE: &str = "dev.usix.axon";
const URL_KEY: &str = "daemon_url";
const TOKEN_KEY: &str = "daemon_token";
const REFRESH_KEY: &str = "daemon_refresh_token";
const EXPIRY_KEY: &str = "daemon_token_expiry";

fn entry(name: &str) -> keyring::Result<Entry> {
    Entry::new(SERVICE, name)
}

/// 내부 전용 읽기 (client 가 토큰·URL 을 읽어 쓴다).
pub(crate) fn read(name: &str) -> Option<String> {
    entry(name).ok()?.get_password().ok()
}

/// 내부 전용: 저장된 Bearer 토큰.
pub(crate) fn token() -> Option<String> {
    read(TOKEN_KEY)
}

#[tauri::command]
pub fn save_daemon_url(url: String) -> Result<(), String> {
    let url = url.trim().trim_end_matches('/').to_string();
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err("URL 은 http:// 또는 https:// 로 시작해야 합니다.".into());
    }
    entry(URL_KEY)
        .and_then(|e| e.set_password(&url))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_daemon_url() -> Result<String, String> {
    read(URL_KEY).ok_or_else(|| "데몬 주소가 설정되지 않았습니다.".into())
}

#[tauri::command]
pub fn save_daemon_token(token: String) -> Result<(), String> {
    entry(TOKEN_KEY)
        .and_then(|e| e.set_password(token.trim()))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn has_daemon_token() -> bool {
    read(TOKEN_KEY).is_some()
}

#[tauri::command]
pub fn clear_daemon() -> Result<(), String> {
    for k in [URL_KEY, TOKEN_KEY, REFRESH_KEY, EXPIRY_KEY] {
        if let Ok(e) = entry(k) {
            let _ = e.delete_credential();
        }
    }
    Ok(())
}

/// 브로커 poll 응답(IdP 토큰 JSON)을 키체인에 저장한다. Bearer = id_token 우선(없으면
/// access_token) — 데몬 JWKS 검증과 정합. refresh·만료도 보관.
pub(crate) fn save_tokens(tok: &serde_json::Value) -> Result<(), String> {
    let access = tok["access_token"].as_str().unwrap_or_default();
    let bearer = tok["id_token"]
        .as_str()
        .filter(|s| !s.is_empty())
        .unwrap_or(access);
    if bearer.is_empty() {
        return Err("토큰 응답에 access_token/id_token 이 없습니다.".into());
    }
    entry(TOKEN_KEY)
        .and_then(|e| e.set_password(bearer))
        .map_err(|e| e.to_string())?;
    if let Some(rt) = tok["refresh_token"].as_str().filter(|s| !s.is_empty()) {
        let _ = entry(REFRESH_KEY).and_then(|e| e.set_password(rt));
    }
    if let Some(exp) = tok["expires_in"].as_u64() {
        let at = now_secs().saturating_add(exp);
        let _ = entry(EXPIRY_KEY).and_then(|e| e.set_password(&at.to_string()));
    }
    Ok(())
}

/// 토큰 3종(access/refresh/expiry) 삭제. 데몬 주소는 유지 — 로그아웃해도 재입력 없게.
pub(crate) fn clear_tokens() -> Result<(), String> {
    for k in [TOKEN_KEY, REFRESH_KEY, EXPIRY_KEY] {
        if let Ok(e) = entry(k) {
            let _ = e.delete_credential();
        }
    }
    Ok(())
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// 저장된 주소로 데몬 헬스를 확인한다. UI 의 "연결 테스트" 버튼용.
/// 토큰이 있으면 Bearer 로 붙여 인증 경로까지 검증한다.
#[tauri::command]
pub async fn ping_daemon() -> Result<String, String> {
    let base = get_daemon_url()?;
    let client = reqwest::Client::builder()
        .user_agent("axon/0.1")
        .build()
        .map_err(|e| e.to_string())?;
    let mut rb = client.get(format!("{base}/status"));
    if let Some(t) = read(TOKEN_KEY) {
        rb = rb.bearer_auth(t);
    }
    let resp = rb.send().await.map_err(|e| format!("연결 실패: {e}"))?;
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    if status.is_success() {
        Ok(if body.is_empty() { "ok".into() } else { body })
    } else {
        Err(format!("데몬 응답 {status}: {body}"))
    }
}

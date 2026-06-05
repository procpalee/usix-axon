//! 구독 entitlement — daemon /me 조회 + (디버그) 토큰 claim 덤프.

use super::config;
use base64::Engine;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Me {
    #[serde(default)]
    pub org: String,
    #[serde(default)]
    pub plan: String,
    #[serde(default)]
    pub features: Vec<String>,
}

/// 로그인 사용자의 구독 등급·활성 기능 조회. plan 배지·hasFeature 의 입력.
#[tauri::command]
pub async fn fetch_me() -> Result<Me, String> {
    let base = config::get_daemon_url()?;
    let token = config::token().ok_or("로그인이 필요합니다.")?;
    let client = reqwest::Client::builder()
        .user_agent("axon/0.1")
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .get(format!("{base}/api/v1/me"))
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    match resp.status().as_u16() {
        200 => resp.json::<Me>().await.map_err(|e| format!("응답 파싱 실패: {e}")),
        401 => Err("인증 만료 — 다시 로그인하세요.".into()),
        other => Err(format!("/me 오류 ({other})")),
    }
}

/// (디버그) 키체인 토큰의 JWT payload claim 을 디코드해 반환. USIX_ORG_CLAIM 진단용.
/// claim 키 확인이 끝나면 이 커맨드와 base64 의존은 제거한다.
#[tauri::command]
pub fn daemon_token_claims() -> Result<serde_json::Value, String> {
    let token = config::token().ok_or("토큰 없음 (로그인 필요)")?;
    let payload = token.split('.').nth(1).ok_or("JWT 형식이 아닙니다")?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .map_err(|e| format!("base64 디코드 실패: {e}"))?;
    serde_json::from_slice(&bytes).map_err(|e| format!("claim 파싱 실패: {e}"))
}

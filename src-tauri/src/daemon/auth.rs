//! 데몬 로그인 — 브라우저 OIDC. 토큰교환은 데몬이 대행하고, 여기선 브라우저 열기→폴링→키체인 저장.
//!
//! 흐름: `POST {daemon}/auth/start` → {authorize_url, poll_token} → 시스템 브라우저
//!   → `GET {daemon}/auth/poll?poll_token` 폴링 → 200 이면 토큰 키체인 저장.

use std::time::Duration;

use super::config;

/// poll 간격·상한. 2초 간격 × 150회 = 5분 내 로그인.
const POLL_INTERVAL_SECS: u64 = 2;
const POLL_MAX_TRIES: u32 = 150;

/// "로그인" 버튼 → 브라우저 OIDC 로그인 → 토큰 키체인 저장.
#[tauri::command]
pub async fn daemon_login() -> Result<(), String> {
    let base = config::get_daemon_url()?;
    let http = reqwest::Client::builder()
        .user_agent("axon/0.1")
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;

    // 1) 로그인 개시 — 데몬이 authorize_url + poll_token 을 만들어 내려준다.
    let start: serde_json::Value = http
        .post(format!("{base}/auth/start"))
        .send()
        .await
        .map_err(|e| format!("로그인 개시 실패: {e}"))?
        .json()
        .await
        .map_err(|e| format!("auth/start 응답 파싱 실패: {e}"))?;
    let authorize_url = start["authorize_url"]
        .as_str()
        .ok_or("authorize_url 없음 — 데몬이 broker 미지원(구버전)일 수 있습니다.")?;
    let poll_token = start["poll_token"]
        .as_str()
        .ok_or("poll_token 없음")?
        .to_string();

    // 2) 시스템 브라우저로 로그인 페이지 열기. 콜백은 데몬이 받으므로 로컬 리스너 불요.
    open_browser(authorize_url);

    // 3) 데몬 폴링 — 토큰이 준비되면(사용자가 로그인 완료) 키체인 저장.
    let poll_url = format!("{base}/auth/poll");
    for _ in 0..POLL_MAX_TRIES {
        tokio::time::sleep(Duration::from_secs(POLL_INTERVAL_SECS)).await;
        let resp = http
            .get(&poll_url)
            .query(&[("poll_token", poll_token.as_str())])
            .send()
            .await
            .map_err(|e| format!("poll 실패: {e}"))?;
        match resp.status().as_u16() {
            200 => {
                let tok: serde_json::Value = resp
                    .json()
                    .await
                    .map_err(|e| format!("토큰 파싱 실패: {e}"))?;
                return config::save_tokens(&tok);
            }
            202 => continue, // pending — 사용자가 아직 로그인 중
            400 => {
                return Err(format!(
                    "로그인 실패: {}",
                    resp.text().await.unwrap_or_default()
                ));
            }
            410 => return Err("로그인 세션이 만료됐습니다 — 다시 시도하세요.".into()),
            other => return Err(format!("예상치 못한 응답({other})")),
        }
    }
    Err("로그인 시간 초과(5분) — 다시 시도하세요.".into())
}

/// 저장된 토큰 삭제 (로그아웃). 데몬 주소는 유지한다.
#[tauri::command]
pub fn daemon_logout() -> Result<(), String> {
    config::clear_tokens()
}

/// 시스템 기본 브라우저로 URL 열기. Tauri IPC 가 아니라 OS 프로세스 직접 spawn(`exec.rs` 선례)
/// → capability 불요. 실패는 무시 — 사용자가 콘솔의 URL 을 직접 열 여지를 남긴다.
fn open_browser(url: &str) {
    if !url.starts_with("https://") {
        return;
    }
    let _ = {
        #[cfg(target_os = "windows")]
        {
            std::process::Command::new("cmd")
                .args(["/C", "start", "", url])
                .spawn()
        }
        #[cfg(target_os = "linux")]
        {
            std::process::Command::new("xdg-open").arg(url).spawn()
        }
    };
}

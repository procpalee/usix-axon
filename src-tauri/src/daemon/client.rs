//! 데몬 HTTP 클라이언트 — SSE 구독 + POST 회신.
//!
//! 엔드포인트:
//!   GET  /api/v1/sessions/{id}/events       SSE 구독 (Last-Event-ID 재연결)
//!   POST /api/v1/sessions/{id}/reply        {content,thinking} → {request_id}
//!   POST /api/v1/sessions/{id}/tool-result  {call_id,output,success}
//!   POST /api/v1/sessions/{id}/permission   {call_id,approved}
//!   POST /api/v1/sessions/{id}/interrupt    진행 중 턴 취소

use std::time::Duration;

use super::config;

pub struct DaemonClient {
    base: String,
    token: Option<String>,
    http: reqwest::Client,
}

impl DaemonClient {
    /// 키체인에 저장된 주소·토큰으로 클라이언트를 만든다. 주소 미설정 시 에러.
    pub fn from_keyring() -> Result<Self, String> {
        let base = config::get_daemon_url()?;
        let http = reqwest::Client::builder()
            .user_agent("axon/0.1")
            .connect_timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| e.to_string())?;
        Ok(Self {
            base,
            token: config::read("daemon_token"),
            http,
        })
    }

    fn auth(&self, rb: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match &self.token {
            Some(t) => rb.bearer_auth(t),
            None => rb,
        }
    }

    fn url(&self, sid: &str, suffix: &str) -> Result<String, String> {
        if sid.is_empty()
            || !sid
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        {
            return Err("잘못된 세션 ID".into());
        }
        Ok(format!("{}/api/v1/sessions/{}/{}", self.base, sid, suffix))
    }

    /// 세션 이벤트 SSE 스트림을 연다. Last-Event-ID 로 재연결 가능.
    pub async fn open_events(
        &self,
        sid: &str,
        last_seq: Option<u64>,
    ) -> Result<reqwest::Response, String> {
        let mut rb = self
            .http
            .get(self.url(sid, "events")?)
            .header("Accept", "text/event-stream");
        if let Some(s) = last_seq {
            rb = rb.header("Last-Event-ID", s.to_string());
        }
        let resp = self.auth(rb).send().await.map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("events 스트림 거부: {}", resp.status()));
        }
        Ok(resp)
    }

    /// 사용자 입력을 턴으로 발사한다. 즉시 request_id 반환(처리는 SSE 로 흘러나옴).
    pub async fn reply(&self, sid: &str, content: &str, thinking: bool) -> Result<String, String> {
        let rb = self.http.post(self.url(sid, "reply")?).json(&serde_json::json!({
            "content": content,
            "thinking": thinking,
        }));
        let resp = self.auth(rb).send().await.map_err(|e| e.to_string())?;
        if resp.status() == reqwest::StatusCode::CONFLICT {
            return Err("이 세션이 다른 곳에서 처리 중입니다 (409).".into());
        }
        let v: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        Ok(v["request_id"].as_str().unwrap_or_default().to_string())
    }

    /// 위임받아 로컬 실행한 도구 결과를 회신한다 (데몬 루프 재개).
    pub async fn tool_result(
        &self,
        sid: &str,
        call_id: &str,
        output: &str,
        success: bool,
    ) -> Result<(), String> {
        let rb = self.http.post(self.url(sid, "tool-result")?).json(&serde_json::json!({
            "call_id": call_id,
            "output": output,
            "success": success,
        }));
        self.auth(rb).send().await.map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn permission(&self, sid: &str, call_id: &str, approved: bool) -> Result<(), String> {
        let rb = self.http.post(self.url(sid, "permission")?).json(&serde_json::json!({
            "call_id": call_id,
            "approved": approved,
        }));
        self.auth(rb).send().await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// 진행 중 턴 취소 (Ctrl+C / Stop 버튼).
    pub async fn interrupt(&self, sid: &str) -> Result<(), String> {
        let rb = self.http.post(self.url(sid, "interrupt")?);
        self.auth(rb).send().await.map_err(|e| e.to_string())?;
        Ok(())
    }
}

//! 데몬 SSE 메시지 → 프론트 UiEvent 매핑.
//!
//! SSE 프레임은 두 모양이 섞여 온다:
//!   1) 래핑:  {"seq":N,"request_id":..,"payload":{"type":..}}   (정상 이벤트)
//!   2) 베어:  {"type":..}                                       (프리앰블/에러)
//!
//! `parse()` 가 둘 다 흡수해 seq·request_id·payload 로 분해한다(`for_turn` 으로 옛 턴 필터).

use serde::Serialize;

/// 프론트(WebView)로 보내는 중립 이벤트. 서버측 용어를 노출하지 않는다.
#[derive(Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum UiEvent {
    Welcome {
        session_id: String,
    },
    Chunk {
        content: String,
    },
    Think {
        content: String,
    },
    Diagnostic {
        content: String,
    },
    /// 클라 위임 도구 실행 결과(tool_execute). detail=인자요약(url/cmd/path), preview=출력 앞부분.
    Tool {
        tool: String,
        ok: bool,
        detail: String,
        preview: String,
    },
    /// 서버 실행 도구 시작(web_search·fetch_url 등). detail=url/query.
    ToolStart {
        tool: String,
        detail: String,
    },
    /// 서버 실행 도구 결과 미리보기.
    ToolResult {
        ok: bool,
        preview: String,
    },
    /// 권한 요청 (Phase 1: 자동 거부 후 사용자에게 표시만).
    Permission {
        tool: String,
        reason: String,
    },
    /// C 하드닝: bash 실행 전 사용자 승인 요청. 프론트가 daemon_approve(call_id, approved) 회신.
    ConfirmExec {
        call_id: String,
        command: String,
    },
    Done,
    Error {
        message: String,
    },
}

/// 파싱된 SSE 프레임. 래핑(SomaEvent)이면 seq·request_id·payload, 베어(전역 프레임)면 payload 만.
pub struct Frame {
    pub seq: Option<u64>,
    pub request_id: Option<String>,
    pub payload: serde_json::Value,
}

/// 래핑/베어 두 모양을 흡수해 프레임으로 분해한다. 파싱 불가(주석·keepalive 등)면 None.
pub fn parse(data: &str) -> Option<Frame> {
    let v: serde_json::Value = serde_json::from_str(data).ok()?;
    match v.get("payload") {
        Some(p) => Some(Frame {
            seq: v.get("seq").and_then(serde_json::Value::as_u64),
            request_id: v
                .get("request_id")
                .and_then(|x| x.as_str())
                .map(str::to_string),
            payload: p.clone(),
        }),
        None => Some(Frame {
            seq: None,
            request_id: None,
            payload: v,
        }),
    }
}

/// 이 프레임이 현재 턴(`expected` request_id)에 속하는가. 래핑 이벤트는 일치할 때만 통과,
/// 베어 전역 프레임(rid=None: Error/ActiveRequests)은 스트림 제어라 항상 통과.
/// cold-subscribe 시 리플레이된 옛 턴 이벤트가 현재 답을 덮거나 조기 종결시키던 누출을 막는다.
pub fn for_turn(request_id: &Option<String>, expected: &str) -> bool {
    match request_id {
        Some(r) => r == expected,
        None => true,
    }
}

/// 객체에서 문자열 필드를 꺼낸다 (없으면 빈 문자열).
pub fn field(v: &serde_json::Value, key: &str) -> String {
    v.get(key)
        .and_then(|x| x.as_str())
        .unwrap_or_default()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_wrapped_and_bare() {
        let wrapped = r#"{"seq":5,"request_id":"r","payload":{"type":"chunk","content":"안녕"}}"#;
        let bare = r#"{"type":"error","message":"x"}"#;
        let w = parse(wrapped).unwrap();
        assert_eq!(w.seq, Some(5));
        assert_eq!(w.request_id.as_deref(), Some("r"));
        assert_eq!(field(&w.payload, "type"), "chunk");
        assert_eq!(field(&w.payload, "content"), "안녕"); // 한글 멀티바이트 보존
        let b = parse(bare).unwrap();
        assert_eq!(b.seq, None);
        assert_eq!(b.request_id, None);
        assert_eq!(field(&b.payload, "type"), "error");
    }

    #[test]
    fn garbage_is_none() {
        assert!(parse(":keep-alive").is_none());
        assert!(parse("").is_none());
    }

    #[test]
    fn for_turn_filters_foreign_passes_bare() {
        // 리플레이된 옛 턴(다른 request_id)은 거부 — 자기소개 턴에 옛 응답 새던 버그.
        assert!(!for_turn(&Some("old".into()), "cur"));
        assert!(for_turn(&Some("cur".into()), "cur"));
        assert!(for_turn(&None, "cur")); // 베어 전역 프레임은 통과
    }
}

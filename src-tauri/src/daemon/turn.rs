//! 턴 루프 — SSE 를 소비해 텍스트는 프론트로 중계하고, tool_execute 는 로컬 도구를 실행해 회신한다.
//! bash 는 동의가 켜져 있어도 명령별 사용자 승인을 받는다(run_bash).
//!
//! 프론트: `daemon_send` 에 Channel<UiEvent> 를 넘기면 스트리밍, ConfirmExec 시 daemon_approve.

use futures_util::StreamExt;
use tauri::async_runtime;
use tauri::ipc::Channel;
use tauri::AppHandle;
use tauri::State;

use super::approvals::Approvals;
use super::client::DaemonClient;
use super::wire::{field, for_turn, parse, UiEvent};
use super::SessionCursor;
use super::{audit, exec, policy, tool_render, tools};

/// 한 턴을 끝까지(Done/Error) 처리한다. 이벤트는 `channel` 로 프론트에 흘린다.
#[tauri::command]
pub async fn daemon_send(
    app: AppHandle,
    session_id: String,
    content: String,
    thinking: bool,
    channel: Channel<UiEvent>,
    approvals: State<'_, Approvals>,
    cursor: State<'_, SessionCursor>,
) -> Result<(), String> {
    let client = DaemonClient::from_keyring()?;
    // 후속 턴은 마지막 seq 이후만 받는다(Last-Event-ID). 없으면 데몬이 버퍼 전체를 replay 해
    // 직전 턴들이 다시 흘러온다. 구독 먼저(이벤트 누락 방지) → 발사.
    let last_seq = cursor.get(&session_id);
    let resp = client.open_events(&session_id, last_seq).await?;
    // reply 가 돌려준 request_id 로 이 턴 이벤트만 통과시킨다 — 리플레이된 옛 턴(그 chunk·done)이
    // 현재 답을 덮거나 조기 종결시키던 누출 차단(데몬 계약: event_for_turn).
    let request_id = client.reply(&session_id, &content, thinking).await?;

    let mut stream = resp.bytes_stream();
    let mut buf: Vec<u8> = Vec::new();
    let mut data: Vec<String> = Vec::new();
    let mut max_seq = last_seq.unwrap_or(0);
    while let Some(chunk) = stream.next().await {
        let bytes = chunk.map_err(|e| e.to_string())?;
        buf.extend_from_slice(&bytes);
        // \n(0x0A)은 UTF-8 멀티바이트 연속바이트(>=0x80)와 겹치지 않으므로, 바이트 버퍼를
        // \n 기준으로 잘라야 한글이 청크 경계에서 깨지지 않는다.
        while let Some(pos) = buf.iter().position(|&b| b == b'\n') {
            let raw: Vec<u8> = buf.drain(..=pos).collect();
            let line = String::from_utf8_lossy(&raw[..raw.len() - 1]);
            let line = line.trim_end_matches('\r');
            if line.is_empty() {
                if data.is_empty() {
                    continue;
                }
                let joined = data.join("\n");
                data.clear();
                let Some(frame) = parse(&joined) else {
                    continue;
                };
                if let Some(s) = frame.seq {
                    max_seq = max_seq.max(s); // 다음 턴 Last-Event-ID high-water
                }
                // 옛 턴(다른 request_id) replay 는 버린다. 베어 전역 프레임은 통과.
                if for_turn(&frame.request_id, &request_id)
                    && handle(
                        &app,
                        &client,
                        &channel,
                        approvals.inner(),
                        &session_id,
                        &frame.payload,
                    )
                    .await?
                {
                    cursor.set(&session_id, max_seq);
                    return Ok(()); // Done/Error = 턴 종료
                }
            } else if let Some(d) = line.strip_prefix("data:") {
                data.push(d.strip_prefix(' ').unwrap_or(d).to_string());
            }
            // ':' 주석(keep-alive), 'id:', 'event:' 은 무시
        }
    }
    cursor.set(&session_id, max_seq);
    Ok(())
}

/// 진행 중 턴 취소 + 대기 중 승인 전부 거부.
#[tauri::command]
pub async fn daemon_cancel(
    session_id: String,
    approvals: State<'_, Approvals>,
) -> Result<(), String> {
    approvals.clear(); // 대기 중 ConfirmExec → recv None → 거부
    DaemonClient::from_keyring()?.interrupt(&session_id).await
}

/// 프론트가 ConfirmExec 에 응답한다 (사용자 승인/거부).
#[tauri::command]
pub async fn daemon_approve(
    call_id: String,
    approved: bool,
    approvals: State<'_, Approvals>,
) -> Result<(), String> {
    if let Some(tx) = approvals.take(&call_id) {
        let _ = tx.send(approved).await;
    }
    Ok(())
}

/// 이벤트 하나 처리. 반환 true = 턴 종료(Done/Error).
async fn handle(
    app: &AppHandle,
    client: &DaemonClient,
    channel: &Channel<UiEvent>,
    approvals: &Approvals,
    sid: &str,
    msg: &serde_json::Value,
) -> Result<bool, String> {
    let emit = |e: UiEvent| {
        let _ = channel.send(e);
    };
    match field(msg, "type").as_str() {
        "welcome" => emit(UiEvent::Welcome {
            session_id: field(msg, "session_id"),
        }),
        "chunk" => emit(UiEvent::Chunk {
            content: field(msg, "content"),
        }),
        "think_chunk" => emit(UiEvent::Think {
            content: field(msg, "content"),
        }),
        "diagnostic" => emit(UiEvent::Diagnostic {
            content: field(msg, "content"),
        }),
        // 서버 실행 도구(web_search·fetch_url 등): 시작 알림 + 결과 미리보기.
        "tool_start" => {
            let tool = field(msg, "tool");
            let args = msg.get("args").cloned().unwrap_or(serde_json::Value::Null);
            emit(UiEvent::ToolStart {
                detail: tool_render::detail(&tool, &args),
                tool,
            });
        }
        "tool_result" => emit(UiEvent::ToolResult {
            ok: msg
                .get("success")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(true),
            preview: tool_render::preview("", &field(msg, "output")),
        }),
        // 클라 위임 도구(bash·read/write/edit_file): 로컬 실행 후 결과 회신.
        "tool_execute" => {
            let call_id = field(msg, "call_id");
            let tool = field(msg, "tool");
            let args = msg.get("args").cloned().unwrap_or(serde_json::Value::Null);
            let detail = tool_render::detail(&tool, &args);
            // bash(C)는 명령별 승인 왕복, 나머지(B)는 정책 디스패치.
            let (output, ok) = if tool == "bash" {
                run_bash(channel, approvals, &call_id, &args).await
            } else {
                tools::dispatch(app, &tool, &args).await
            };
            let preview = tool_render::preview(&tool, &output);
            emit(UiEvent::Tool {
                tool,
                ok,
                detail,
                preview,
            });
            client.tool_result(sid, &call_id, &output, ok).await?;
        }
        "permission_request" => {
            let call_id = field(msg, "call_id");
            emit(UiEvent::Permission {
                tool: field(msg, "tool"),
                reason: field(msg, "reason"),
            });
            client.permission(sid, &call_id, false).await?;
        }
        "done" => {
            emit(UiEvent::Done);
            return Ok(true);
        }
        "error" => {
            emit(UiEvent::Error {
                message: field(msg, "message"),
            });
            return Ok(true);
        }
        // 그 외 이벤트는 소비하지 않고 버린다.
        _ => {}
    }
    Ok(false)
}

/// bash(C) 명령별 승인 왕복: shell 동의 확인 → ConfirmExec emit → 사용자 응답 대기 → 실행/거부.
/// 응답은 daemon_approve(Some) 또는 daemon_cancel/드롭(None=거부)으로 온다.
/// ⚠️ 데몬 ToolExecute 30s 타임아웃과 결합 — 승인이 느리면 데몬이 먼저 TIMEOUT 처리할 수 있다.
async fn run_bash(
    channel: &Channel<UiEvent>,
    approvals: &Approvals,
    call_id: &str,
    args: &serde_json::Value,
) -> (String, bool) {
    if !policy::shell_enabled() {
        return (
            "셸 실행 비활성 — 설정에서 실험기능(C)에 동의해야 합니다.".into(),
            false,
        );
    }
    let command = args["command"].as_str().unwrap_or("").to_string();
    let (tx, mut rx) = async_runtime::channel::<bool>(1);
    approvals.insert(call_id.to_string(), tx);
    let _ = channel.send(UiEvent::ConfirmExec {
        call_id: call_id.to_string(),
        command: command.clone(),
    });
    let approved = rx.recv().await.unwrap_or(false);
    approvals.take(call_id);
    if !approved {
        return ("사용자가 명령 실행을 거부했습니다.".into(), false);
    }
    let (out, ok) = exec::run(&command);
    audit::log(&command, ok);
    (out, ok)
}

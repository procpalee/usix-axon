//! 로컬 도구 실행 디스패치 — 데몬이 `tool_execute` 로 요청한 파일 I/O 를 수행한다.
//! bash 는 명령별 승인이 필요해 turn.rs(`run_bash`)에서 처리한다.
//! 미지원 도구는 `(_, false)` 로 돌려준다.

use tauri::AppHandle;

use super::fs_tools;
use super::policy;

/// 도구를 실행하고 `(출력, 성공여부)` 를 돌려준다.
/// 거부(success=false)해도 데몬은 결과를 받아 루프를 이어간다(에이전트가 적응).
pub async fn dispatch(app: &AppHandle, tool: &str, args: &serde_json::Value) -> (String, bool) {
    match tool {
        "read_file" | "write_file" | "edit_file" => {
            let Some(ws) = policy::workspace() else {
                return ("워크스페이스 미설정 — 폴더를 먼저 지정하세요(설정).".into(), false);
            };
            match tool {
                "read_file" => fs_tools::read_file(&ws, args),
                "write_file" => fs_tools::write_file(&ws, args),
                _ => fs_tools::edit_file(&ws, args),
            }
        }
        "compute_footing" => super::workpaper::compute_footing().await,
        "search_company" => super::fetch_tools::search_company(app, args).await,
        "load_financials" => super::fetch_tools::load_financials(app, args).await,
        other => (format!("도구 '{other}' 는 이 클라이언트에서 지원하지 않습니다."), false),
    }
}

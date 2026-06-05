//! 셸 실행 감사 로그 — 승인·실행된 명령을 append (베스트에포트, 실패 무시).
//!
//! 위치: `$XDG_STATE_HOME/usix-axon/shell-audit.log` (없으면 `$HOME/.local/state/...`).
//! 형식: `<unix_ts>\t<ok|fail>\t<command>` (개행은 공백 치환). Windows 는 경로 없으면 skip.

use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn log(command: &str, ok: bool) {
    let Some(path) = log_path() else { return };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        let _ = writeln!(
            f,
            "{ts}\t{}\t{}",
            if ok { "ok" } else { "fail" },
            command.replace('\n', " ")
        );
    }
}

fn log_path() -> Option<std::path::PathBuf> {
    let base = std::env::var_os("XDG_STATE_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".local/state")))?;
    Some(base.join("usix-axon").join("shell-audit.log"))
}

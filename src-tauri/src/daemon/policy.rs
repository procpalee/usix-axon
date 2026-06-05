//! 도구 실행 정책 — 워크스페이스 경로 제한.
//!
//! 데몬이 위임한 파일 I/O 는 사용자가 고른 워크스페이스 내부에서만 허용한다(`jail`).
//! ⚠️ 보안 임계 코드.

use std::path::{Component, Path, PathBuf};

use keyring::Entry;

const SERVICE: &str = "dev.usix.axon";
const WS_KEY: &str = "daemon_workspace";
const SHELL_KEY: &str = "daemon_shell"; // 셸 실행 동의 플래그

fn ws_entry() -> keyring::Result<Entry> {
    Entry::new(SERVICE, WS_KEY)
}

/// 현재 워크스페이스 루트 (미설정 시 None → 파일 도구 비활성).
pub(crate) fn workspace() -> Option<PathBuf> {
    Some(PathBuf::from(ws_entry().ok()?.get_password().ok()?))
}

/// 요청 경로를 워크스페이스 안으로 가둔다. 탈출 시 Err.
///
/// 방어 3중 (각각 다른 공격을 막아 단독으론 불충분):
///
///   1) 절대경로 거부 — 워크스페이스 기준 상대 경로만.
///   2) `..`(ParentDir)·Prefix 등 비정상 컴포넌트 거부 (`.`/`./`는 무시).
///   3) 부모 디렉터리 `canonicalize` → 워크스페이스 하위 확인 (심링크 탈출 차단).
///
/// 새 파일(write)도 부모는 존재해야 하므로 부모 기준으로 검사한다(`canonicalize`는 존재 경로만 가능).
pub fn jail(ws: &Path, requested: &str) -> Result<PathBuf, String> {
    let rel = Path::new(requested);
    if rel.is_absolute() {
        return Err(format!("절대경로 거부: {requested}"));
    }
    // Normal 컴포넌트만 수집: '.' 무시, '..'/RootDir/Prefix 거부.
    let mut clean = PathBuf::new();
    for c in rel.components() {
        match c {
            Component::Normal(s) => clean.push(s),
            Component::CurDir => {}
            Component::ParentDir => return Err(format!("'..' 경로 거부: {requested}")),
            _ => return Err(format!("허용되지 않는 경로 컴포넌트: {requested}")),
        }
    }
    let ws_real = ws
        .canonicalize()
        .map_err(|e| format!("워크스페이스 접근 불가({}): {e}", ws.display()))?;
    if clean.as_os_str().is_empty() {
        return Ok(ws_real); // 워크스페이스 루트 자체
    }
    let target = ws_real.join(&clean);
    let parent = target.parent().ok_or("상위 경로 없음")?;
    let parent_real = parent
        .canonicalize()
        .map_err(|e| format!("상위 디렉터리 없음({}): {e}", parent.display()))?;
    if !parent_real.starts_with(&ws_real) {
        return Err(format!("워크스페이스 밖 거부: {requested}"));
    }
    Ok(parent_real.join(clean.file_name().ok_or("파일명 없음")?))
}

/// 워크스페이스 지정 (폴더 피커 결과). 존재하는 디렉터리만 허용.
#[tauri::command]
pub fn set_workspace(path: String) -> Result<(), String> {
    let p = PathBuf::from(path.trim());
    if !p.is_dir() {
        return Err("디렉터리가 존재하지 않습니다.".into());
    }
    let canonical = p.canonicalize().map_err(|e| e.to_string())?;
    ws_entry()
        .and_then(|e| e.set_password(&canonical.to_string_lossy()))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_workspace() -> Option<String> {
    workspace().map(|p| p.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn clear_workspace() -> Result<(), String> {
    if let Ok(e) = ws_entry() {
        let _ = e.delete_credential();
    }
    Ok(())
}

/// 셸 실행(C) 동의 여부.
pub(crate) fn shell_enabled() -> bool {
    Entry::new(SERVICE, SHELL_KEY)
        .ok()
        .and_then(|e| e.get_password().ok())
        .as_deref()
        == Some("1")
}

/// 셸 실행(C) 동의 토글. ⚠️ 켜면 데몬이 임의 명령 실행 가능 — 프론트에서 경고 후 호출.
#[tauri::command]
pub fn set_shell_enabled(enabled: bool) -> Result<(), String> {
    let e = Entry::new(SERVICE, SHELL_KEY).map_err(|x| x.to_string())?;
    if enabled {
        e.set_password("1").map_err(|x| x.to_string())
    } else {
        let _ = e.delete_credential();
        Ok(())
    }
}

#[tauri::command]
pub fn shell_is_enabled() -> bool {
    shell_enabled()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_ws() -> PathBuf {
        let base = std::env::temp_dir().join("axon_jail_test");
        let _ = std::fs::create_dir_all(base.join("sub"));
        base.canonicalize().unwrap()
    }

    #[test]
    fn accepts_inside_workspace() {
        let ws = tmp_ws();
        assert!(jail(&ws, "y.txt").is_ok());
        assert!(jail(&ws, "sub/x.txt").is_ok());
        assert!(jail(&ws, "./sub/x.txt").is_ok());
        assert_eq!(jail(&ws, ".").unwrap(), ws); // 루트 자체
    }

    #[test]
    fn rejects_parent_traversal() {
        let ws = tmp_ws();
        assert!(jail(&ws, "../escape.txt").is_err());
        assert!(jail(&ws, "sub/../../escape.txt").is_err());
    }

    #[test]
    fn rejects_absolute_path() {
        let ws = tmp_ws();
        assert!(jail(&ws, "/etc/passwd").is_err());
    }
}

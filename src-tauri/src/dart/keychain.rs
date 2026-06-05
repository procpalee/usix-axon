//! OS 키체인 — OpenDART 인증키 보관. 프론트엔 존재 여부만 노출(키 값은 안 내려감).

use keyring::Entry;

const SERVICE: &str = "dev.usix.axon";
const KEY_NAME: &str = "opendart_api_key";

fn entry() -> keyring::Result<Entry> {
    Entry::new(SERVICE, KEY_NAME)
}

/// 내부 전용: opendart 가 키를 읽어 API 호출에 쓴다 (프론트 노출 X).
pub(crate) fn get_key() -> keyring::Result<String> {
    entry()?.get_password()
}

#[tauri::command]
pub fn save_opendart_key(key: String) -> Result<(), String> {
    entry()
        .and_then(|e| e.set_password(&key))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn has_opendart_key() -> bool {
    entry().and_then(|e| e.get_password()).is_ok()
}

#[tauri::command]
pub fn delete_opendart_key() -> Result<(), String> {
    entry()
        .and_then(|e| e.delete_credential())
        .map_err(|e| e.to_string())
}

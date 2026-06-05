//! 세션별 마지막 seq 커서. 후속 턴 /events 구독 시 Last-Event-ID 로 실어, 데몬이
//! 버퍼 전체를 replay(직전 턴 chunk·done 누출)하지 않게 한다. Tauri State 로 주입.

use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Default)]
pub struct SessionCursor(Mutex<HashMap<String, u64>>);

impl SessionCursor {
    /// 이 세션에서 마지막으로 본 seq (없으면 None = 첫 연결 → 데몬 cold replay).
    pub fn get(&self, session_id: &str) -> Option<u64> {
        self.0.lock().ok()?.get(session_id).copied()
    }

    /// high-water seq 갱신 (단조 증가 — 감소는 무시).
    pub fn set(&self, session_id: &str, seq: u64) {
        if let Ok(mut map) = self.0.lock() {
            let cur = map.entry(session_id.to_string()).or_insert(0);
            *cur = (*cur).max(seq);
        }
    }
}

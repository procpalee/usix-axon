//! 대기 중 셸 실행 승인 (Tauri managed state).
//!
//! turn 루프가 bash 승인 대기 시 call_id별 `Sender` 를 보관하고, `daemon_approve` 가
//! 꺼내 사용자 결정을 회신한다. `daemon_cancel` 은 전부 드롭해 거부 처리한다.

use std::collections::HashMap;
use std::sync::Mutex;

use tauri::async_runtime::Sender;

#[derive(Default)]
pub struct Approvals(Mutex<HashMap<String, Sender<bool>>>);

impl Approvals {
    /// 승인 대기 등록 (turn 루프).
    pub fn insert(&self, call_id: String, tx: Sender<bool>) {
        if let Ok(mut m) = self.0.lock() {
            m.insert(call_id, tx);
        }
    }

    /// 대기 항목 꺼내기 (daemon_approve / turn 정리).
    pub fn take(&self, call_id: &str) -> Option<Sender<bool>> {
        self.0.lock().ok()?.remove(call_id)
    }

    /// 전부 비움 → 대기 중 승인 거부 (daemon_cancel).
    pub fn clear(&self) {
        if let Ok(mut m) = self.0.lock() {
            m.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_take_clear() {
        let a = Approvals::default();
        let (tx, _rx) = tauri::async_runtime::channel::<bool>(1);
        a.insert("c1".into(), tx);
        assert!(a.take("c1").is_some());
        assert!(a.take("c1").is_none()); // 이미 꺼냄 → 중복 take 없음
        let (tx2, _rx2) = tauri::async_runtime::channel::<bool>(1);
        a.insert("c2".into(), tx2);
        a.clear();
        assert!(a.take("c2").is_none()); // clear 후 거부
    }
}

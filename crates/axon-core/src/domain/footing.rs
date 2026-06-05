//! 합계검증 — 도메인 타입. 검증 한 줄(항목·기간·계산값·기대값·일치여부).
//! 계산 로직은 `service::footing`. footing 은 tol=0(정확) — tie-out(톨러런스)은 별도(후속).

use crate::domain::money::Money;

/// 합계검증 한 줄. `computed`(보고된 총계) vs `expected`(하위 합).
#[derive(Debug, Clone)]
pub struct FootingCheck {
    pub name: &'static str,
    pub period: i32,
    pub computed: Money,
    pub expected: Money,
    pub ok: bool,
}

impl FootingCheck {
    /// 차이 (computed − expected). 0 이면 일치.
    pub fn diff(&self) -> Money {
        self.computed - self.expected
    }

    /// 표시 — 일치면 "OK", 불일치면 차이 금액.
    pub fn display(&self) -> String {
        if self.ok {
            "OK".to_string()
        } else {
            self.diff().fmt_comma()
        }
    }
}

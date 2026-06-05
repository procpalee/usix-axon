//! 재무비율 — 도메인 타입. 비율 한 줄(이름·기간·값·단위).
//! 값 None = 계산 불가(필요 계정 누락 또는 분모 0). 계산 로직은 `service::ratio`.

use rust_decimal::Decimal;

/// 비율 단위 — 표시·해석용.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    /// 백분율 (%)
    Percent,
    /// 배수 (배) — 회전율·이자보상배율 등 (후속).
    Times,
}

/// 재무비율 한 줄.
#[derive(Debug, Clone)]
pub struct Ratio {
    pub name: &'static str,
    /// 산식 (예: "유동자산 / 유동부채") — 무엇을 무엇으로 나눴는지 화면 표시.
    pub formula: &'static str,
    pub period: i32,
    /// 계산 결과. 필요 계정 누락·분모 0 이면 None.
    pub value: Option<Decimal>,
    pub unit: Unit,
}

impl Ratio {
    /// 표시 문자열 — "200%"·"12.34%"·"1.2배", 계산 불가 시 "—". (불필요한 소수 0 제거.)
    pub fn display(&self) -> String {
        match self.value {
            Some(v) => match self.unit {
                Unit::Percent => format!("{}%", v.normalize()),
                Unit::Times => format!("{}배", v.normalize()),
            },
            None => "—".to_string(),
        }
    }
}

//! Money — 금액 타입. f64 금지, rust_decimal 기반.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Money(pub Decimal);

impl Money {
    pub const ZERO: Money = Money(Decimal::ZERO);

    /// 회계 금액 문자열을 파싱. 콤마·공백 제거, 빈 값/"-"는 0.
    /// (OpenDART 는 "1,234,567" 같은 콤마 포함 문자열로 금액을 준다.)
    pub fn parse(raw: &str) -> Money {
        let cleaned: String = raw
            .chars()
            .filter(|c| *c != ',' && !c.is_whitespace())
            .collect();
        if cleaned.is_empty() || cleaned == "-" {
            return Money::ZERO;
        }
        Decimal::from_str(&cleaned).map(Money).unwrap_or(Money::ZERO)
    }

    /// 천단위 콤마 포맷 (455905980000000 → "455,905,980,000,000"). 소수부·부호 보존.
    pub fn fmt_comma(&self) -> String {
        let s = self.0.to_string();
        let (sign, rest) = match s.strip_prefix('-') {
            Some(r) => ("-", r),
            None => ("", s.as_str()),
        };
        let (int_part, frac) = rest.split_once('.').unwrap_or((rest, ""));
        let len = int_part.len();
        let mut grouped = String::with_capacity(len + len / 3);
        for (i, c) in int_part.chars().enumerate() {
            if i > 0 && (len - i) % 3 == 0 {
                grouped.push(',');
            }
            grouped.push(c);
        }
        let mut out = format!("{sign}{grouped}");
        if !frac.is_empty() {
            out.push('.');
            out.push_str(frac);
        }
        out
    }
}

impl std::ops::Add for Money {
    type Output = Money;
    fn add(self, rhs: Money) -> Money {
        Money(self.0 + rhs.0)
    }
}

impl std::ops::Sub for Money {
    type Output = Money;
    fn sub(self, rhs: Money) -> Money {
        Money(self.0 - rhs.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_strips_commas_and_handles_blanks() {
        assert_eq!(Money::parse("1,234,567"), Money(Decimal::from(1_234_567)));
        assert_eq!(Money::parse("-1,000"), Money(Decimal::from(-1_000)));
        assert_eq!(Money::parse(""), Money::ZERO);
        assert_eq!(Money::parse("-"), Money::ZERO);
        assert_eq!(Money::parse(" 500 "), Money(Decimal::from(500)));
    }

    #[test]
    fn fmt_comma_groups_by_three() {
        assert_eq!(
            Money(Decimal::from(455_905_980_000_000_i64)).fmt_comma(),
            "455,905,980,000,000"
        );
        assert_eq!(Money(Decimal::from(1_234)).fmt_comma(), "1,234");
        assert_eq!(Money(Decimal::from(100)).fmt_comma(), "100");
        assert_eq!(Money(Decimal::from(-1_000)).fmt_comma(), "-1,000");
        assert_eq!(Money::ZERO.fmt_comma(), "0");
    }
}

//! 재무비율 계산 (use case) — Fact(결정론) 위에서. LLM 0.
//! 계정은 **concept_id**(XBRL `account_id`)로 잡는다 — `account_nm` 은 회사마다 달라
//! 신뢰 불가. 필요 계정 누락·분모 0 은 None(계산 불가)으로 흡수 → 일부 비율만 나와도
//! 전체가 실패하지 않는다.

use crate::domain::fact::Fact;
use crate::domain::ratio::{Ratio, Unit};
use rust_decimal::Decimal;

use crate::domain::concept;

/// concept_id·기간의 금액. 없으면 None.
fn amount(facts: &[Fact], concept_id: &str, period: i32) -> Option<Decimal> {
    facts
        .iter()
        .find(|f| f.concept_id == concept_id && f.period == period)
        .map(|f| f.value.0)
}

/// num/den × scale, 소수 2자리. 분모 0·피연산자 누락은 None.
fn div(num: Option<Decimal>, den: Option<Decimal>, scale: Decimal) -> Option<Decimal> {
    match (num, den) {
        (Some(n), Some(d)) if !d.is_zero() => Some((n / d * scale).round_dp(2)),
        _ => None,
    }
}

/// 백분율 비율 한 줄 빌더.
fn pct(
    name: &'static str,
    formula: &'static str,
    period: i32,
    num: Option<Decimal>,
    den: Option<Decimal>,
) -> Ratio {
    Ratio {
        name,
        formula,
        period,
        value: div(num, den, Decimal::from(100)),
        unit: Unit::Percent,
    }
}

/// 모든 기간의 재무비율 (유동성·안정성·수익성). 최신 연도부터.
pub fn compute_ratios(facts: &[Fact]) -> Vec<Ratio> {
    use std::collections::BTreeSet;
    let periods: BTreeSet<i32> = facts.iter().map(|f| f.period).collect();

    let mut out = Vec::new();
    for &p in periods.iter().rev() {
        let ca = amount(facts, concept::CURRENT_ASSETS, p);
        let cl = amount(facts, concept::CURRENT_LIABILITIES, p);
        let liab = amount(facts, concept::LIABILITIES, p);
        let eq = amount(facts, concept::EQUITY, p);
        let asset = amount(facts, concept::ASSETS, p);
        let rev = amount(facts, concept::REVENUE, p);
        let ni = amount(facts, concept::PROFIT_LOSS, p);
        let oi = amount(facts, concept::OPERATING_INCOME, p);

        out.push(pct("유동비율", "유동자산 / 유동부채", p, ca, cl));
        out.push(pct("부채비율", "부채총계 / 자본총계", p, liab, eq));
        out.push(pct("자기자본비율", "자본총계 / 자산총계", p, eq, asset));
        out.push(pct("영업이익률", "영업이익 / 매출액", p, oi, rev));
        out.push(pct("순이익률", "당기순이익 / 매출액", p, ni, rev));
        out.push(pct("ROA(총자산순이익률)", "당기순이익 / 자산총계", p, ni, asset));
        out.push(pct("ROE(자기자본순이익률)", "당기순이익 / 자본총계", p, ni, eq));
        // 주: ROE·ROA 는 기말 기준. 평균((기초+기말)/2)은 전기 Fact 가 있을 때의 후속 개선.
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::money::Money;
    use crate::domain::statement::Statement;

    fn f(concept_id: &str, period: i32, v: &str) -> Fact {
        Fact {
            statement: Statement::Bs,
            concept_id: concept_id.to_string(),
            account_nm: String::new(),
            period,
            value: Money::parse(v),
        }
    }

    #[test]
    fn computes_liquidity_and_debt_ratios() {
        let facts = vec![
            f(concept::CURRENT_ASSETS, 2023, "300"),
            f(concept::CURRENT_LIABILITIES, 2023, "150"),
            f(concept::LIABILITIES, 2023, "400"),
            f(concept::EQUITY, 2023, "200"),
        ];
        let r = compute_ratios(&facts);
        let get = |name: &str| r.iter().find(|x| x.name == name).unwrap().value;
        assert_eq!(get("유동비율"), Some(Decimal::from(200))); // 300/150*100
        assert_eq!(get("부채비율"), Some(Decimal::from(200))); // 400/200*100
    }

    #[test]
    fn missing_account_or_zero_denominator_is_none() {
        let facts = vec![f(concept::CURRENT_ASSETS, 2023, "300")]; // 유동부채 없음
        let cur = compute_ratios(&facts)
            .into_iter()
            .find(|x| x.name == "유동비율")
            .unwrap();
        assert_eq!(cur.value, None);
        assert_eq!(cur.display(), "—");
    }

    #[test]
    fn alias_lets_named_facts_feed_ratios() {
        // account_id 없이 계정명만으로 들어와도 melt 가 표준키를 채워 비율이 계산된다.
        let facts: Vec<Fact> = [("유동자산", "300"), ("유동부채", "150")]
            .iter()
            .flat_map(|(nm, v)| {
                Fact::melt(
                    Statement::Bs,
                    "",
                    nm,
                    2023,
                    [Money::parse(v), Money::parse("0"), Money::parse("0")],
                )
            })
            .collect();
        let cur = compute_ratios(&facts)
            .into_iter()
            .find(|x| x.name == "유동비율")
            .unwrap();
        assert_eq!(cur.value, Some(Decimal::from(200))); // 300/150*100
    }

    #[test]
    fn display_strips_trailing_zeros() {
        let facts = vec![
            f(concept::CURRENT_ASSETS, 2023, "300"),
            f(concept::CURRENT_LIABILITIES, 2023, "150"),
        ];
        let cur = compute_ratios(&facts)
            .into_iter()
            .find(|x| x.name == "유동비율")
            .unwrap();
        assert_eq!(cur.display(), "200%"); // 200.00 → "200%"
    }
}

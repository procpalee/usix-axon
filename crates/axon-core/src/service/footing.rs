//! 합계검증 (use case) — 표준 회계 항등식을 concept_id 로 검증. LLM 0, tol=0(정확).
//! 회계등식(자산=부채+자본)·소계 일관성(유동+비유동=총계)을 Fact 위에서 대사한다.
//! 전체 계정트리 roll-up(parent=Σchildren)은 XBRL 택사노미(계정계층) 오면 확장.

use crate::domain::fact::Fact;
use crate::domain::footing::FootingCheck;
use crate::domain::money::Money;

use crate::domain::concept;

/// concept_id·기간의 금액. 없으면 None.
fn amount(facts: &[Fact], concept_id: &str, period: i32) -> Option<Money> {
    facts
        .iter()
        .find(|f| f.concept_id == concept_id && f.period == period)
        .map(|f| f.value)
}

/// `computed == Σparts` 검증 한 줄. 필요 계정 누락이면 None(검증 불가 → 건너뜀).
fn check(
    name: &'static str,
    period: i32,
    computed: Option<Money>,
    parts: &[Option<Money>],
) -> Option<FootingCheck> {
    let computed = computed?;
    let mut expected = Money::ZERO;
    for p in parts {
        expected = expected + (*p)?; // 한 조각이라도 없으면 검증 불가
    }
    Some(FootingCheck {
        name,
        period,
        computed,
        expected,
        ok: computed == expected,
    })
}

/// 모든 기간의 합계검증 (회계등식·소계 일관성). 검증 가능한 항목만 반환.
pub fn verify_footing(facts: &[Fact]) -> Vec<FootingCheck> {
    use std::collections::BTreeSet;
    let periods: BTreeSet<i32> = facts.iter().map(|f| f.period).collect();

    let mut out = Vec::new();
    for &p in periods.iter().rev() {
        let assets = amount(facts, concept::ASSETS, p);
        let liab = amount(facts, concept::LIABILITIES, p);
        let eq = amount(facts, concept::EQUITY, p);

        out.extend(check("회계등식 (자산=부채+자본)", p, assets, &[liab, eq]));
        out.extend(check(
            "자산총계 = 유동+비유동",
            p,
            assets,
            &[
                amount(facts, concept::CURRENT_ASSETS, p),
                amount(facts, concept::NONCURRENT_ASSETS, p),
            ],
        ));
        out.extend(check(
            "부채총계 = 유동+비유동",
            p,
            liab,
            &[
                amount(facts, concept::CURRENT_LIABILITIES, p),
                amount(facts, concept::NONCURRENT_LIABILITIES, p),
            ],
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn accounting_equation_holds_and_breaks() {
        let ok = vec![
            f(concept::ASSETS, 2023, "1000"),
            f(concept::LIABILITIES, 2023, "600"),
            f(concept::EQUITY, 2023, "400"),
        ];
        let c = verify_footing(&ok)
            .into_iter()
            .find(|c| c.name.starts_with("회계등식"))
            .unwrap();
        assert!(c.ok);
        assert_eq!(c.diff(), Money::ZERO);

        let bad = vec![
            f(concept::ASSETS, 2023, "1000"),
            f(concept::LIABILITIES, 2023, "600"),
            f(concept::EQUITY, 2023, "300"), // 자본 100 부족
        ];
        let c = verify_footing(&bad)
            .into_iter()
            .find(|c| c.name.starts_with("회계등식"))
            .unwrap();
        assert!(!c.ok);
        assert_eq!(c.diff(), Money::parse("100")); // 1000 − 900
        assert_eq!(c.display(), "100");
    }

    #[test]
    fn missing_account_skips_check() {
        let facts = vec![f(concept::ASSETS, 2023, "1000")]; // 부채·자본 없음
        assert!(verify_footing(&facts)
            .iter()
            .all(|c| !c.name.starts_with("회계등식")));
    }
}

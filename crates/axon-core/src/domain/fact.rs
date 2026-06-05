//! Fact — 재무제표 데이터의 tidy(long) 단위. (제표, 계정, 연도) = 값 하나.
//!
//! OpenDART 한 줄은 당기/전기/전전기 3개 금액을 가진 wide 형태다. 이걸 그대로
//! 들고 다니면 "여러 개년 분석"이 3차원 테이블로 번진다. 대신 wide → long 으로
//! **melt** 해서 Fact 를 쌓고, 표시할 때만 `Table::pivot` 으로 되편다.
//! 차원(회사·연결/별도)이 늘어도 grid 차원이 아니라 Fact 의 필드(=slicer)가 는다.

use crate::domain::money::Money;
use crate::domain::statement::Statement;
use serde::{Deserialize, Serialize};

/// 재무제표 한 칸. `concept_id`(XBRL `account_id`) 는 회사·연도 간 비율 분석의 키 —
/// 계정명(`account_nm`)은 회사마다 제각각이라 신뢰할 수 없다. 비표준 계정이면 빈 문자열.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fact {
    pub statement: Statement,
    pub concept_id: String,
    pub account_nm: String,
    pub period: i32,
    pub value: Money,
    // company·basis(CFS/OFS) 는 멀티회사·연결/별도 비교가 붙을 때 추가 (지금은 단일·CFS 고정).
}

impl Fact {
    /// wide 한 줄(당기/전기/전전기) → long Fact 들. `base_year` 가 당기.
    /// 금액 0(빈칸/"-")도 Fact 로 남긴다 — pivot 단계에서 "0"과 "해당 연도 미수록"을 구분.
    pub fn melt(
        statement: Statement,
        concept_id: &str,
        account_nm: &str,
        base_year: i32,
        amounts: [Money; 3],
    ) -> Vec<Fact> {
        // concept_id 가 비면 계정명 별칭으로 표준키 보강(openbb transform_data: vendor→canonical).
        // 이 한 지점이 OpenDART Fact 의 유일한 생성 경로 → 하류(pivot·ratio·footing) 전부 표준키를 본다.
        let concept_id = if concept_id.is_empty() {
            crate::domain::alias::resolve(account_nm).unwrap_or("")
        } else {
            concept_id
        };
        amounts
            .into_iter()
            .enumerate()
            .map(|(i, value)| Fact {
                statement,
                concept_id: concept_id.to_string(),
                account_nm: account_nm.to_string(),
                period: base_year - i as i32,
                value,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn melt_explodes_three_periods_descending() {
        let facts = Fact::melt(
            Statement::Bs,
            "ifrs-full_Assets",
            "자산총계",
            2023,
            [Money::parse("300"), Money::parse("200"), Money::parse("100")],
        );
        assert_eq!(facts.len(), 3);
        assert_eq!(facts[0].period, 2023);
        assert_eq!(facts[1].period, 2022);
        assert_eq!(facts[2].period, 2021);
        assert_eq!(facts[0].concept_id, "ifrs-full_Assets");
        assert_eq!(facts[0].value, Money::parse("300"));
    }

    #[test]
    fn melt_fills_concept_from_alias_when_empty() {
        // account_id 비어도 계정명 별칭으로 표준 concept_id 보강(openbb transform_data).
        let facts = Fact::melt(
            Statement::Bs,
            "",
            "유동자산",
            2023,
            [Money::parse("1"), Money::parse("2"), Money::parse("3")],
        );
        assert_eq!(facts[0].concept_id, crate::domain::concept::CURRENT_ASSETS);
    }

    #[test]
    fn melt_keeps_explicit_concept_id() {
        // account_id 가 있으면 별칭은 건드리지 않는다(원본 우선).
        let facts = Fact::melt(
            Statement::Bs,
            "ifrs-full_Assets",
            "아무이름",
            2023,
            [Money::parse("1"), Money::parse("2"), Money::parse("3")],
        );
        assert_eq!(facts[0].concept_id, "ifrs-full_Assets");
    }

    #[test]
    fn melt_unknown_name_stays_empty() {
        // 별칭에 없는 비표준 계정은 빈 concept_id 유지(폴백은 pivot 의 정규화 계정명).
        let facts = Fact::melt(
            Statement::Bs,
            "",
            "비표준계정xyz",
            2023,
            [Money::parse("1"), Money::parse("2"), Money::parse("3")],
        );
        assert_eq!(facts[0].concept_id, "");
    }
}

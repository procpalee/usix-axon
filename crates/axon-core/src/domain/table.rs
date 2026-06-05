//! Table — 범용 행/열 *뷰*. Fact(long) 를 pivot 한 표시·export 공통 입력.
//! 도메인 모델이 아니라 뷰다 — Money 는 여기서 처음 String 으로 렌더된다 (분석은
//! 이 위가 아니라 Fact/Money 위에서 돈다).

use crate::domain::{fact::Fact, footing::FootingCheck, money::Money, ratio::Ratio, statement::Statement};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Table {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
    /// 행별 merge_key(concept_id) — 트리 섹션 매핑·표준라벨 조회용. 표시·export 제외, 빈=미사용.
    #[serde(default)]
    pub keys: Vec<String>,
}

impl Table {
    /// Fact 들 → 계정×연도 표. 컬럼 = [재무제표, 계정, ...연도(내림차순)].
    /// 행 식별·셀 저장 키 = (statement, account_nm): 같은 이름이라도 제표가 다르면 다른 행
    /// (재무상태표 "기타" vs 손익계산서 "기타"가 안 섞이게). 같은 칸 중복은 마지막 값.
    pub fn pivot(facts: &[Fact]) -> Table {
        use crate::domain::account::merge_key;
        use std::collections::{BTreeSet, HashMap, HashSet};

        let mut years: BTreeSet<i32> = BTreeSet::new();
        // 행 식별 = (제표, merge_key): concept_id 우선 병합(계정 개명·비표준 표기에 강함).
        // 표시 계정명은 첫 등장값을 쓴다.
        let mut order: Vec<(Statement, String, String)> = Vec::new();
        let mut seen: HashSet<(Statement, String)> = HashSet::new();
        let mut cell: HashMap<(Statement, String, i32), Money> = HashMap::new();

        for f in facts {
            let mkey = merge_key(&f.concept_id, &f.account_nm);
            if seen.insert((f.statement, mkey.clone())) {
                order.push((f.statement, mkey.clone(), f.account_nm.clone()));
            }
            years.insert(f.period);
            cell.insert((f.statement, mkey, f.period), f.value);
        }
        // 제표 순서로 그룹화 (BS→IS→CIS→CF→SCE). 안정 정렬이라 제표 내 계정 순서는 보존.
        order.sort_by_key(|(stmt, _, _)| stmt.order());

        let year_list: Vec<i32> = years.iter().rev().copied().collect();
        let mut headers = vec!["재무제표".to_string(), "계정".to_string()];
        headers.extend(year_list.iter().map(|y| y.to_string()));

        let rows = order
            .iter()
            .map(|(stmt, mkey, display)| {
                let mut row = vec![stmt.name().to_string(), display.clone()];
                row.extend(year_list.iter().map(|y| {
                    cell.get(&(*stmt, mkey.clone(), *y))
                        .map(Money::fmt_comma)
                        .unwrap_or_default()
                }));
                row
            })
            .collect();
        // 행별 merge_key(concept_id) — pivot 순서와 1:1. 트리 섹션 매핑·표준라벨 조회용.
        let keys = order.iter().map(|(_, mkey, _)| mkey.clone()).collect();

        Table { headers, rows, keys }
    }

    /// 재무비율 목록 → 지표×연도 표. (`service::ratio::compute_ratios` 결과의 뷰.)
    pub fn from_ratios(ratios: &[Ratio]) -> Table {
        use std::collections::{BTreeSet, HashMap, HashSet};
        let mut periods: BTreeSet<i32> = BTreeSet::new();
        let mut order: Vec<(&'static str, &'static str)> = Vec::new();
        let mut seen: HashSet<&'static str> = HashSet::new();
        let mut cell: HashMap<(&'static str, i32), String> = HashMap::new();

        for r in ratios {
            if seen.insert(r.name) {
                order.push((r.name, r.formula));
            }
            periods.insert(r.period);
            cell.insert((r.name, r.period), r.display());
        }

        let year_list: Vec<i32> = periods.iter().rev().copied().collect();
        let mut headers = vec!["지표".to_string(), "산식".to_string()];
        headers.extend(year_list.iter().map(|y| y.to_string()));
        let rows = order
            .iter()
            .map(|(name, formula)| {
                let mut row = vec![name.to_string(), formula.to_string()];
                row.extend(
                    year_list
                        .iter()
                        .map(|y| cell.get(&(*name, *y)).cloned().unwrap_or_else(|| "—".to_string())),
                );
                row
            })
            .collect();
        Table { headers, rows, keys: Vec::new() }
    }

    /// 합계검증 결과 → 항목×연도 표. (일치="OK", 불일치=차이 금액.)
    pub fn from_footing(checks: &[FootingCheck]) -> Table {
        use std::collections::{BTreeSet, HashMap, HashSet};
        let mut periods: BTreeSet<i32> = BTreeSet::new();
        let mut order: Vec<&'static str> = Vec::new();
        let mut seen: HashSet<&'static str> = HashSet::new();
        let mut cell: HashMap<(&'static str, i32), String> = HashMap::new();

        for c in checks {
            if seen.insert(c.name) {
                order.push(c.name);
            }
            periods.insert(c.period);
            cell.insert((c.name, c.period), c.display());
        }

        let year_list: Vec<i32> = periods.iter().rev().copied().collect();
        let mut headers = vec!["검증 항목".to_string()];
        headers.extend(year_list.iter().map(|y| y.to_string()));
        let rows = order
            .iter()
            .map(|name| {
                let mut row = vec![name.to_string()];
                row.extend(
                    year_list
                        .iter()
                        .map(|y| cell.get(&(*name, *y)).cloned().unwrap_or_else(|| "—".to_string())),
                );
                row
            })
            .collect();
        Table { headers, rows, keys: Vec::new() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::money::Money;

    fn fact(stmt: Statement, acc: &str, period: i32, v: &str) -> Fact {
        Fact {
            statement: stmt,
            concept_id: String::new(),
            account_nm: acc.to_string(),
            period,
            value: Money::parse(v),
        }
    }

    #[test]
    fn pivots_multi_year_into_columns() {
        let facts = vec![
            fact(Statement::Bs, "자산총계", 2022, "300"),
            fact(Statement::Bs, "자산총계", 2021, "200"),
            fact(Statement::Bs, "자산총계", 2020, "100"),
            fact(Statement::Bs, "자산총계", 2019, "90"),
            fact(Statement::Bs, "자산총계", 2018, "80"),
            fact(Statement::Bs, "자산총계", 2017, "70"),
        ];
        let t = Table::pivot(&facts);
        assert_eq!(
            t.headers,
            vec!["재무제표", "계정", "2022", "2021", "2020", "2019", "2018", "2017"]
        );
        assert_eq!(
            t.rows[0],
            vec!["재무상태표", "자산총계", "300", "200", "100", "90", "80", "70"]
        );
    }

    #[test]
    fn same_account_name_in_different_statements_not_merged() {
        // "기타"가 재무상태표·손익계산서 양쪽에 있어도 셀이 섞이면 안 된다.
        let facts = vec![
            fact(Statement::Bs, "기타", 2022, "10"),
            fact(Statement::Is, "기타", 2022, "20"),
        ];
        let t = Table::pivot(&facts);
        assert_eq!(t.rows.len(), 2);
        let bs = t.rows.iter().find(|r| r[0] == "재무상태표").unwrap();
        let is = t.rows.iter().find(|r| r[0] == "손익계산서").unwrap();
        assert_eq!(bs[2], "10"); // 2022 재무상태표 기타
        assert_eq!(is[2], "20"); // 2022 손익계산서 기타
    }

    #[test]
    fn pivot_groups_by_statement_order() {
        // 입력이 뒤섞여(SCE→CF→IS→BS) 들어와도 출력은 제표 순서로 그룹.
        let facts = vec![
            fact(Statement::Sce, "자본금", 2023, "5"),
            fact(Statement::Cf, "영업현금", 2023, "3"),
            fact(Statement::Is, "매출", 2023, "2"),
            fact(Statement::Bs, "자산총계", 2023, "1"),
        ];
        let t = Table::pivot(&facts);
        let stmts: Vec<&str> = t.rows.iter().map(|r| r[0].as_str()).collect();
        assert_eq!(stmts, vec!["재무상태표", "손익계산서", "현금흐름표", "자본변동표"]);
    }

    #[test]
    fn pivot_merges_synonyms_via_alias() {
        // concept_id 없이 동의어("매출액" 2023 + "영업수익" 2022)여도 한 행(둘 다 REVENUE).
        let facts = vec![
            fact(Statement::Is, "매출액", 2023, "100"),
            fact(Statement::Is, "영업수익", 2022, "90"),
        ];
        let t = Table::pivot(&facts);
        assert_eq!(t.rows.len(), 1); // 별칭으로 한 행
        assert_eq!(t.rows[0], vec!["손익계산서", "매출액", "100", "90"]); // 표시는 첫 등장명
    }

    #[test]
    fn pivot_merges_by_concept_id_across_rename() {
        // 같은 concept_id 인데 계정명이 연도마다 다름(개명) → 한 행으로 병합.
        let facts = vec![
            Fact {
                statement: Statement::Bs,
                concept_id: "ifrs-full_Assets".into(),
                account_nm: "자산총계".into(),
                period: 2023,
                value: Money::parse("100"),
            },
            Fact {
                statement: Statement::Bs,
                concept_id: "ifrs-full_Assets".into(),
                account_nm: "총자산".into(),
                period: 2022,
                value: Money::parse("90"),
            },
        ];
        let t = Table::pivot(&facts);
        assert_eq!(t.rows.len(), 1); // 개명됐어도 concept_id 로 한 행
        assert_eq!(t.rows[0], vec!["재무상태표", "자산총계", "100", "90"]); // 표시는 첫 등장명
    }
}

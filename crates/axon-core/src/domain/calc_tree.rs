//! 계산연결(calculation linkbase) 트리 — 계정 부모-자식·표시순서·합계가중치.
//! XBRL `cal.xml` 파싱 결과의 도메인 표현(어댑터가 arc 를 채운다). 완전 다단계 재무제표
//! 트리 + footing(parent = Σ weight·child)의 기반. 상장사 linkbase 다수에서 추출하면
//! 비상장·임포트 데이터에 재사용할 표준 계층 사전이 된다. [[axon-xbrl-linkbase-tree]]
//! ⚠️ 데이터는 회사 공시 XBRL(런타임 fetch→캐시) — repo 커밋 금지.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// 한 부모 아래 자식 하나 — concept_id + 표시순서 + 합계가중치.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalcChild {
    pub concept_id: String,
    /// 표시 순서(돈이 아니라 정렬 키 — f64 허용). cal.xml 의 `order`(현금=1.0 …).
    pub order: f64,
    /// 합계 가중치(+1 가산 / -1 차감). footing 계산용.
    pub weight: i32,
}

/// parent concept_id → order 순 children. 계정 계층 전체.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CalcTree {
    children: HashMap<String, Vec<CalcChild>>,
}

impl CalcTree {
    /// (parent, child, order, weight) arc 들로 트리 구성. 자식은 order 순 정렬.
    pub fn from_arcs(arcs: Vec<(String, String, f64, i32)>) -> CalcTree {
        let mut children: HashMap<String, Vec<CalcChild>> = HashMap::new();
        for (parent, concept_id, order, weight) in arcs {
            children.entry(parent).or_default().push(CalcChild {
                concept_id,
                order,
                weight,
            });
        }
        for v in children.values_mut() {
            v.sort_by(|a, b| a.order.partial_cmp(&b.order).unwrap_or(std::cmp::Ordering::Equal));
        }
        CalcTree { children }
    }

    /// 직속 자식들(order 순). 없으면 빈 슬라이스.
    pub fn children_of(&self, concept_id: &str) -> &[CalcChild] {
        self.children.get(concept_id).map_or(&[], Vec::as_slice)
    }

    /// 루트들 — 부모로만 나오고 자식으론 안 나온 concept (제표 최상위: 자산·부채·자본·매출 등).
    pub fn roots(&self) -> Vec<&str> {
        let as_child: HashSet<&str> = self
            .children
            .values()
            .flatten()
            .map(|c| c.concept_id.as_str())
            .collect();
        self.children
            .keys()
            .map(String::as_str)
            .filter(|k| !as_child.contains(k))
            .collect()
    }

    pub fn is_empty(&self) -> bool {
        self.children.is_empty()
    }

    /// 여러 회사 CalcTree → 표준 트리(다수결). 상장사 다수에서 추출해 비상장·임포트에 재사용.
    /// 각 계정(child)의 parent = 최빈, order = 중앙값, weight = 최빈. 소수 의견(이상 분류)은 버려진다.
    pub fn consensus(trees: &[CalcTree]) -> CalcTree {
        let mut parent_votes: HashMap<&str, HashMap<&str, u32>> = HashMap::new();
        let mut orders: HashMap<&str, Vec<f64>> = HashMap::new();
        let mut weight_votes: HashMap<&str, HashMap<i32, u32>> = HashMap::new();
        for tree in trees {
            for (parent, kids) in &tree.children {
                for ch in kids {
                    let c = ch.concept_id.as_str();
                    *parent_votes.entry(c).or_default().entry(parent.as_str()).or_default() += 1;
                    orders.entry(c).or_default().push(ch.order);
                    *weight_votes.entry(c).or_default().entry(ch.weight).or_default() += 1;
                }
            }
        }
        let mut arcs = Vec::new();
        for (child, pv) in &parent_votes {
            let parent = pv
                .iter()
                .max_by_key(|(_, n)| **n)
                .map(|(p, _)| (*p).to_string())
                .unwrap_or_default();
            let mut os = orders.get(child).cloned().unwrap_or_default();
            os.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let order = os.get(os.len() / 2).copied().unwrap_or(0.0); // 중앙값
            let weight = weight_votes
                .get(child)
                .and_then(|w| w.iter().max_by_key(|(_, n)| **n).map(|(wt, _)| *wt))
                .unwrap_or(1);
            arcs.push((parent, (*child).to_string(), order, weight));
        }
        CalcTree::from_arcs(arcs)
    }

    /// footing 검증 — parent 값 ≠ Σ(weight·child) 인 계정 목록. 회사가 XBRL 에 신고한
    /// 모든 계층 합계를 결정론으로 대조한다(합계 오류·분식 단서). 값 없는 child 는 합산 제외.
    pub fn footing_violations(&self, values: &HashMap<String, Decimal>) -> Vec<FootingViolation> {
        let mut out = Vec::new();
        for (parent, kids) in &self.children {
            let Some(&actual) = values.get(parent) else {
                continue; // parent 값 미수록 → 검증 불가, skip
            };
            let mut sum = Decimal::ZERO;
            let mut any = false;
            for ch in kids {
                if let Some(&v) = values.get(&ch.concept_id) {
                    sum += Decimal::from(ch.weight) * v;
                    any = true;
                }
            }
            if any && sum != actual {
                out.push(FootingViolation {
                    concept_id: parent.clone(),
                    expected: sum,
                    actual,
                    diff: actual - sum,
                });
            }
        }
        out
    }
}

/// footing 위반 한 건 — 신고값(actual)과 자식 합계(expected)의 불일치.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FootingViolation {
    pub concept_id: String,
    pub expected: Decimal, // Σ(weight·child)
    pub actual: Decimal,   // 신고된 parent 값
    pub diff: Decimal,     // actual - expected (0 이 아니면 위반)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_tree_sorted_by_order() {
        // 입력이 순서 뒤섞여도 order 로 정렬: 유동자산(1) 먼저, 비유동자산(2) 뒤.
        let arcs = vec![
            ("ifrs-full_Assets".into(), "ifrs-full_NoncurrentAssets".into(), 2.0, 1),
            ("ifrs-full_Assets".into(), "ifrs-full_CurrentAssets".into(), 1.0, 1),
            ("ifrs-full_CurrentAssets".into(), "ifrs-full_CashAndCashEquivalents".into(), 1.0, 1),
        ];
        let t = CalcTree::from_arcs(arcs);
        let kids = t.children_of("ifrs-full_Assets");
        assert_eq!(kids.len(), 2);
        assert_eq!(kids[0].concept_id, "ifrs-full_CurrentAssets"); // order 1 = 먼저
        assert_eq!(kids[1].concept_id, "ifrs-full_NoncurrentAssets");
        // 현금이 유동자산 아래.
        assert_eq!(
            t.children_of("ifrs-full_CurrentAssets")[0].concept_id,
            "ifrs-full_CashAndCashEquivalents"
        );
    }

    #[test]
    fn roots_are_top_level_only() {
        let arcs = vec![
            ("ifrs-full_Assets".into(), "ifrs-full_CurrentAssets".into(), 1.0, 1),
            ("ifrs-full_CurrentAssets".into(), "ifrs-full_Cash".into(), 1.0, 1),
        ];
        let t = CalcTree::from_arcs(arcs);
        assert_eq!(t.roots(), vec!["ifrs-full_Assets"]); // 자식인 적 없는 최상위만
    }

    #[test]
    fn consensus_majority_parent_and_median_order() {
        // 3개 회사: 2개는 Cash→유동자산(order 1·2), 1개는 Cash→기타(order 9).
        let t1 = CalcTree::from_arcs(vec![("CurrentAssets".into(), "Cash".into(), 1.0, 1)]);
        let t2 = CalcTree::from_arcs(vec![("CurrentAssets".into(), "Cash".into(), 2.0, 1)]);
        let t3 = CalcTree::from_arcs(vec![("OtherAssets".into(), "Cash".into(), 9.0, 1)]);
        let c = CalcTree::consensus(&[t1, t2, t3]);
        // 다수결: Cash 는 CurrentAssets 아래(2표 > 1표), 소수 parent(OtherAssets) 버려짐.
        let kids = c.children_of("CurrentAssets");
        assert_eq!(kids.len(), 1);
        assert_eq!(kids[0].concept_id, "Cash");
        assert!(c.children_of("OtherAssets").is_empty());
        assert_eq!(kids[0].order, 2.0); // 중앙값(1,2,9 → 2)
    }

    #[test]
    fn negative_weight_preserved() {
        // 차감 항목(대손충당금 등)은 weight -1 — footing 계산용.
        let arcs = vec![("p".into(), "allowance".into(), 1.0, -1)];
        let t = CalcTree::from_arcs(arcs);
        assert_eq!(t.children_of("p")[0].weight, -1);
    }

    fn dec(n: i64) -> Decimal {
        Decimal::from(n)
    }

    #[test]
    fn footing_flags_mismatch() {
        let t = CalcTree::from_arcs(vec![
            ("Assets".into(), "CurrentAssets".into(), 1.0, 1),
            ("Assets".into(), "NoncurrentAssets".into(), 1.0, 1),
        ]);
        let mut v: HashMap<String, Decimal> = HashMap::new();
        v.insert("Assets".into(), dec(100));
        v.insert("CurrentAssets".into(), dec(60));
        v.insert("NoncurrentAssets".into(), dec(30)); // 60+30=90 ≠ 100
        let viol = t.footing_violations(&v);
        assert_eq!(viol.len(), 1);
        assert_eq!(viol[0].concept_id, "Assets");
        assert_eq!(viol[0].diff, dec(10)); // 100 - 90
    }

    #[test]
    fn footing_ok_when_balanced() {
        let t = CalcTree::from_arcs(vec![
            ("Assets".into(), "CurrentAssets".into(), 1.0, 1),
            ("Assets".into(), "NoncurrentAssets".into(), 1.0, 1),
        ]);
        let mut v: HashMap<String, Decimal> = HashMap::new();
        v.insert("Assets".into(), dec(100));
        v.insert("CurrentAssets".into(), dec(60));
        v.insert("NoncurrentAssets".into(), dec(40)); // 60+40=100 일치
        assert!(t.footing_violations(&v).is_empty());
    }

    #[test]
    fn footing_respects_negative_weight() {
        // 매출총이익 = 매출 - 매출원가 (weight -1).
        let t = CalcTree::from_arcs(vec![
            ("GrossProfit".into(), "Revenue".into(), 1.0, 1),
            ("GrossProfit".into(), "CostOfSales".into(), 1.0, -1),
        ]);
        let mut v: HashMap<String, Decimal> = HashMap::new();
        v.insert("GrossProfit".into(), dec(30));
        v.insert("Revenue".into(), dec(100));
        v.insert("CostOfSales".into(), dec(70)); // 100 - 70 = 30 일치
        assert!(t.footing_violations(&v).is_empty());
    }
}

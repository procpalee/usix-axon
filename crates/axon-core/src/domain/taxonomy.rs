//! XBRL 표준계정체계(택사노미) — 표준 라벨·계정 우주(universe)·트리 골격.
//! OpenDART `xbrlTaxonomy.json`(IFRS 기반 표준계정과목) 파싱 결과의 도메인 표현.
//! ⚠️ **데이터는 런타임 fetch→로컬 캐시. repo 커밋 금지(IFRS 저작권).** [[axon-ref-borrow-plan]]
//!
//! 용도: ① 표준 한글/영문 라벨 조회(회사별로 흔들리는 `account_nm` 대신 표준명) ② `is_standard`
//! 판정(비표준·`dart_` 계정 식별) ③ 라벨→concept_id 역인덱스 = 대형 alias 소스([[alias]] 보강).
//! 계산 계층(`parent`)은 응답에 있으면 트리, 없으면 평면 — calc linkbase 유무는 어댑터가 채운다.

use crate::domain::account::normalize;
use crate::domain::statement::Statement;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 표준계정 한 개 (택사노미 한 행).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxonomyConcept {
    /// 표준계정ID — 재무제표(fnlttSinglAcntAll)와 맞추려 `ifrs-full_*` 형식으로 정규화 적재.
    pub concept_id: String,
    /// 표준 한글 라벨.
    pub label_ko: String,
    /// 표준 영문 라벨.
    pub label_en: String,
    /// K-IFRS 근거 문단("K-IFRS 1001 문단 60") — 감사 조서 레퍼런스.
    pub ifrs_ref: String,
    /// 그룹 헤더(값 없는 `*Abstract` 분류 노드) — 트리 가지/섹션 경계.
    pub is_abstract: bool,
    /// 상위 concept_id — calc 링크가 있을 때만(xbrlTaxonomy 엔 없어 현재 None=평면).
    pub parent: Option<String>,
    /// 표시 순서(응답 순서 = presentation 순서).
    pub ord: i32,
}

/// 한 재무제표(`Statement`)의 표준계정 집합 + 트리/라벨 질의.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Taxonomy {
    pub statement: Statement,
    pub concepts: Vec<TaxonomyConcept>,
}

impl Taxonomy {
    /// concept_id 의 표준 한글 라벨. 없으면 None.
    pub fn label_ko(&self, concept_id: &str) -> Option<&str> {
        self.concepts
            .iter()
            .find(|c| c.concept_id == concept_id)
            .map(|c| c.label_ko.as_str())
    }

    /// concept_id 가 이 택사노미(표준)에 존재하는가. (비표준·확장계정 식별용.)
    pub fn is_standard(&self, concept_id: &str) -> bool {
        self.concepts.iter().any(|c| c.concept_id == concept_id)
    }

    /// 루트 계정들(상위 없음) — ord 순. parent 가 다 None 이면 전체가 루트(평면).
    pub fn roots(&self) -> Vec<&TaxonomyConcept> {
        let mut v: Vec<&TaxonomyConcept> = self.concepts.iter().filter(|c| c.parent.is_none()).collect();
        v.sort_by_key(|c| c.ord);
        v
    }

    /// 직속 자식들 — ord 순.
    pub fn children_of(&self, concept_id: &str) -> Vec<&TaxonomyConcept> {
        let mut v: Vec<&TaxonomyConcept> = self
            .concepts
            .iter()
            .filter(|c| c.parent.as_deref() == Some(concept_id))
            .collect();
        v.sort_by_key(|c| c.ord);
        v
    }

    /// 정규화 라벨 → concept_id 역인덱스. 택사노미 자체가 거대한 별칭 사전이다([[alias]] 보강).
    /// 같은 정규화 라벨이 둘이면 먼저 등장한 것을 남긴다(표준이 우선 배치된다는 가정).
    pub fn alias_index(&self) -> HashMap<String, String> {
        let mut idx = HashMap::with_capacity(self.concepts.len());
        for c in &self.concepts {
            idx.entry(normalize(&c.label_ko)).or_insert_with(|| c.concept_id.clone());
        }
        idx
    }

    /// concept_id → 섹션 라벨 맵 (제표>섹션>계정 트리의 섹션 레이어).
    /// presentation 순서로 순회 — 첫 abstract(제표 루트)는 스킵, 이후 abstract 를 만나면 현재
    /// 섹션 갱신, 비-abstract 계정은 현재 섹션에 귀속. 첫 섹션 이전 계정(IS 본문 등)은 섹션 없음
    /// (맵에 미수록=제표 직속). BS=자산/부채/자본, CF/SCE=섹션 0(루트만).
    pub fn section_map(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        let mut current: Option<&str> = None;
        let mut seen_root = false;
        for c in &self.concepts {
            if c.is_abstract {
                if seen_root {
                    current = Some(&c.label_ko);
                } else {
                    seen_root = true; // 첫 abstract = 제표 루트, 섹션 아님
                }
            } else if let Some(sec) = current {
                map.insert(c.concept_id.clone(), sec.to_string());
            }
        }
        map
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn concept(id: &str, ko: &str, parent: Option<&str>, ord: i32) -> TaxonomyConcept {
        TaxonomyConcept {
            concept_id: id.to_string(),
            label_ko: ko.to_string(),
            label_en: String::new(),
            ifrs_ref: String::new(),
            is_abstract: false,
            parent: parent.map(str::to_string),
            ord,
        }
    }

    fn concept_ab(id: &str, ko: &str, ord: i32) -> TaxonomyConcept {
        TaxonomyConcept {
            concept_id: id.to_string(),
            label_ko: ko.to_string(),
            label_en: String::new(),
            ifrs_ref: String::new(),
            is_abstract: true,
            parent: None,
            ord,
        }
    }

    fn sample() -> Taxonomy {
        Taxonomy {
            statement: Statement::Bs,
            concepts: vec![
                concept("ifrs-full_Assets", "자산총계", None, 0),
                concept("ifrs-full_CurrentAssets", "유동자산", Some("ifrs-full_Assets"), 1),
                concept("ifrs-full_NoncurrentAssets", "비유동자산", Some("ifrs-full_Assets"), 2),
            ],
        }
    }

    #[test]
    fn label_and_standard_lookup() {
        let t = sample();
        assert_eq!(t.label_ko("ifrs-full_CurrentAssets"), Some("유동자산"));
        assert!(t.is_standard("ifrs-full_Assets"));
        assert!(!t.is_standard("dart_madeup")); // 비표준
    }

    #[test]
    fn tree_roots_and_children() {
        let t = sample();
        let roots = t.roots();
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].concept_id, "ifrs-full_Assets");
        let kids = t.children_of("ifrs-full_Assets");
        assert_eq!(kids.len(), 2);
        assert_eq!(kids[0].concept_id, "ifrs-full_CurrentAssets"); // ord 순
    }

    #[test]
    fn alias_index_maps_normalized_label_to_concept() {
        let t = sample();
        let idx = t.alias_index();
        assert_eq!(idx.get("유동자산").map(String::as_str), Some("ifrs-full_CurrentAssets"));
    }

    #[test]
    fn flat_taxonomy_all_roots() {
        // parent 가 다 None(=calc 링크 없는 평면 응답)이면 전부 루트.
        let t = Taxonomy {
            statement: Statement::Is,
            concepts: vec![
                concept("ifrs-full_Revenue", "수익(매출액)", None, 0),
                concept("ifrs-full_ProfitLoss", "당기순이익", None, 1),
            ],
        };
        assert_eq!(t.roots().len(), 2);
    }

    #[test]
    fn section_map_skips_root_groups_by_section() {
        // BS: 첫 abstract(재무상태표)=루트 스킵, 자산/부채=섹션, 그 아래 계정은 섹션 귀속.
        let t = Taxonomy {
            statement: Statement::Bs,
            concepts: vec![
                concept_ab("ifrs-full_StatementOfFinancialPositionAbstract", "재무상태표", 0),
                concept_ab("ifrs-full_AssetsAbstract", "자산", 1),
                concept("ifrs-full_CurrentAssets", "유동자산", None, 2),
                concept("ifrs-full_Cash", "현금", None, 3),
                concept_ab("ifrs-full_LiabilitiesAbstract", "부채", 4),
                concept("ifrs-full_CurrentLiabilities", "유동부채", None, 5),
            ],
        };
        let m = t.section_map();
        assert_eq!(m.get("ifrs-full_CurrentAssets").map(String::as_str), Some("자산"));
        assert_eq!(m.get("ifrs-full_Cash").map(String::as_str), Some("자산"));
        assert_eq!(m.get("ifrs-full_CurrentLiabilities").map(String::as_str), Some("부채"));
        assert!(!m.contains_key("ifrs-full_AssetsAbstract")); // abstract 자체는 멤버 아님
        assert!(!m.contains_key("ifrs-full_StatementOfFinancialPositionAbstract")); // 루트도 아님
    }

    #[test]
    fn section_map_empty_when_only_root() {
        // CF/SCE: abstract 가 루트 하나뿐 → 섹션 0(전부 제표 직속).
        let t = Taxonomy {
            statement: Statement::Cf,
            concepts: vec![
                concept_ab("ifrs-full_StatementOfCashFlowsAbstract", "현금흐름표", 0),
                concept("ifrs-full_CashFlowsFromOperatingActivities", "영업활동현금흐름", None, 1),
            ],
        };
        assert!(t.section_map().is_empty());
    }
}

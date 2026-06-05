//! 계정명 별칭 사전 — 표기가 흔들리는 한글 계정명을 표준 concept_id 로 해소.
//! (openbb adapter `alias_dict` 발상 clean-room: vendor 키→canonical 을 소스 경계에서 1회.)
//!
//! `concept_id`(XBRL account_id)가 비어 [`normalize`] 폴백마저 회사별로 갈릴 때,
//! *알려진* 동의어("매출액"="영업수익"="수익(매출액)")면 표준키로 못박는다.
//! ⚠️ 값 기반 휴리스틱이 아니라 **이름 정확매칭**만 — footing 을 깨는 추정 매칭은 안 한다.
//! 모호한 일반어(자산·부채 단독 등)는 일부러 뺐다(오매칭이 누락보다 위험).

use crate::domain::account::normalize;
use crate::domain::concept;

/// 계정명 → 표준 concept_id. 알려진 별칭이 아니면 None.
/// 입력은 원시 계정명(내부에서 [`normalize`] 로 괄호주석·번호·공백 제거 후 매칭).
pub fn resolve(account_nm: &str) -> Option<&'static str> {
    Some(match normalize(account_nm).as_str() {
        // 재무상태표 — "총계" 형태는 모호하지 않다.
        "자산총계" => concept::ASSETS,
        "유동자산" => concept::CURRENT_ASSETS,
        "비유동자산" => concept::NONCURRENT_ASSETS,
        "부채총계" => concept::LIABILITIES,
        "유동부채" => concept::CURRENT_LIABILITIES,
        "비유동부채" => concept::NONCURRENT_LIABILITIES,
        "자본총계" => concept::EQUITY,
        // 손익계산서 — 매출/수익 동의어. "수익(매출액)"→normalize→"수익"(K-IFRS 표준 라벨).
        "매출액" | "매출" | "영업수익" | "수익" => concept::REVENUE,
        // "당기순이익(손실)"→normalize→"당기순이익". 분기/반기 라벨도 같은 개념.
        "당기순이익" | "당기순손익" | "분기순이익" | "반기순이익" => concept::PROFIT_LOSS,
        "영업이익" | "영업손익" => concept::OPERATING_INCOME,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_known_aliases_to_concept() {
        assert_eq!(resolve("자산총계"), Some(concept::ASSETS));
        assert_eq!(resolve("영업수익"), Some(concept::REVENUE));
        assert_eq!(resolve("매출액"), Some(concept::REVENUE));
    }

    #[test]
    fn resolves_through_normalization() {
        // 괄호주석·번호·공백이 붙어도 normalize 후 매칭.
        assert_eq!(resolve("Ⅰ. 유동자산"), Some(concept::CURRENT_ASSETS));
        assert_eq!(resolve("수익(매출액)"), Some(concept::REVENUE));
        assert_eq!(resolve("당기순이익(손실)"), Some(concept::PROFIT_LOSS));
        assert_eq!(resolve("영업이익(손실)"), Some(concept::OPERATING_INCOME));
    }

    #[test]
    fn unknown_or_ambiguous_returns_none() {
        assert_eq!(resolve("매출원가"), None); // "매출"로 오매칭되면 안 됨
        assert_eq!(resolve("매출총이익"), None);
        assert_eq!(resolve("기타포괄손익"), None);
        assert_eq!(resolve("이름없는계정"), None);
    }
}

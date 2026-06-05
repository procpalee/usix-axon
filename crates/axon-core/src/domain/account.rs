//! 계정명 처리 — 정규화·병합키. (dart-fss `extract_account_title` 발상 clean-room.)
//! account_nm 은 회사·연도마다 표기가 흔들려(번호·괄호주석·공백) 직접 비교가 불안정.
//! 병합·매칭은 concept_id 가 1순위, 없을 때만 정규화 계정명을 폴백으로 쓴다.

/// 계정명 정규화 — 괄호주석([…]·(…)·<…>) 제거 + 공백 제거 + 선행 번호매김 제거.
/// *표시용이 아니라 매칭용* 키. (concept_id 가 비었을 때의 폴백.)
pub fn normalize(name: &str) -> String {
    // 1) 괄호 주석 제거 + 공백 제거.
    let mut s = String::with_capacity(name.len());
    let mut depth = 0i32;
    for c in name.chars() {
        match c {
            '(' | '[' | '<' | '（' | '【' => depth += 1,
            ')' | ']' | '>' | '）' | '】' => depth = (depth - 1).max(0),
            _ if depth == 0 && !c.is_whitespace() => s.push(c),
            _ => {}
        }
    }
    strip_enum_prefix(&s)
}

/// 선행 번호매김 제거: "1." "12)" "Ⅰ." "가." 같은 [번호/로마자/한글자] + 구분자(./)).
/// 단 "가지급금"처럼 구분자 없이 한글로 시작하는 진짜 계정명은 깎지 않는다.
fn strip_enum_prefix(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() && (chars[i].is_ascii_digit() || "ⅠⅡⅢⅣⅤⅥⅦⅧⅨⅩ".contains(chars[i])) {
        i += 1;
    }
    if i > 0 && i < chars.len() && (chars[i] == '.' || chars[i] == ')') {
        return chars[i + 1..].iter().collect();
    }
    // 단일 글자(가./a) 등) — 두 번째 문자가 구분자일 때만.
    if chars.len() >= 2 && (chars[1] == '.' || chars[1] == ')') {
        return chars[2..].iter().collect();
    }
    s.to_string()
}

/// 다년·회사 병합 키 — concept_id 우선(신뢰) → 별칭 사전 → 정규화 계정명 폴백.
/// account_nm 으로만 병합하면 계정 개명·비표준 표기에 취약하다(dart-fss 핵심 차용).
/// 별칭 단계로 동의어("매출액"="영업수익")까지 한 키로 묶는다([[alias]], openbb 차용).
pub fn merge_key(concept_id: &str, account_nm: &str) -> String {
    if !concept_id.is_empty() {
        return concept_id.to_string();
    }
    if let Some(canonical) = crate::domain::alias::resolve(account_nm) {
        return canonical.to_string();
    }
    normalize(account_nm)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_strips_brackets_numbering_whitespace() {
        assert_eq!(normalize("Ⅰ. 유동자산"), "유동자산");
        assert_eq!(normalize("1. 현금및현금성자산 (주2)"), "현금및현금성자산");
        assert_eq!(normalize("가. 자본금"), "자본금");
        assert_eq!(normalize("기타 (상세)"), "기타");
        assert_eq!(normalize("가지급금"), "가지급금"); // 구분자 없으면 안 깎임
    }

    #[test]
    fn merge_key_chain_concept_alias_normalize() {
        use crate::domain::concept;
        // 1순위: concept_id 가 있으면 그대로.
        assert_eq!(merge_key("ifrs-full_Assets", "자산총계"), "ifrs-full_Assets");
        // 2순위: concept_id 비면 별칭 사전("Ⅰ. 유동자산" → 표준키).
        assert_eq!(merge_key("", "Ⅰ. 유동자산"), concept::CURRENT_ASSETS);
        // 3순위: 별칭에도 없으면 정규화 계정명 폴백.
        assert_eq!(merge_key("", "Ⅰ. 비표준계정"), "비표준계정");
    }
}

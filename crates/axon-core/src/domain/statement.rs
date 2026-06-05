//! 재무제표 구분 — OpenDART `sj_div` 코드 → 타입드 enum.
//! 한글 이름(`sj_nm`) 문자열로 분기하지 않고 이 enum 으로 분류·match 한다:
//! 5종을 다 처리 안 하면 `match` 가 컴파일을 거부하고, "손익게산서" 같은 오타는
//! 파싱 즉시 죽는다 (stringly-typed 였다면 런타임까지 살아남음).

use serde::{Deserialize, Serialize};

/// 재무제표 종류. `fnlttSinglAcntAll` 의 `sj_div` 5종.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Statement {
    /// 재무상태표 (BS)
    Bs,
    /// 손익계산서 (IS)
    Is,
    /// 포괄손익계산서 (CIS)
    Cis,
    /// 현금흐름표 (CF)
    Cf,
    /// 자본변동표 (SCE)
    Sce,
}

impl Statement {
    /// 표시·정렬용 고정 순서.
    pub const ALL: [Statement; 5] = [
        Statement::Bs,
        Statement::Is,
        Statement::Cis,
        Statement::Cf,
        Statement::Sce,
    ];

    /// `sj_div` 코드 → enum. 알 수 없는 코드는 `None` (이 엔드포인트는 5종만 반환).
    pub fn from_sj_div(code: &str) -> Option<Statement> {
        match code {
            "BS" => Some(Statement::Bs),
            "IS" => Some(Statement::Is),
            "CIS" => Some(Statement::Cis),
            "CF" => Some(Statement::Cf),
            "SCE" => Some(Statement::Sce),
            _ => None,
        }
    }

    /// 한글 재무제표명 (화면·엑셀 표시용).
    pub fn name(&self) -> &'static str {
        match self {
            Statement::Bs => "재무상태표",
            Statement::Is => "손익계산서",
            Statement::Cis => "포괄손익계산서",
            Statement::Cf => "현금흐름표",
            Statement::Sce => "자본변동표",
        }
    }

    /// 표시·그룹 정렬 순서: 재무상태표→손익→포괄손익→현금흐름→자본변동.
    /// (현금흐름표를 자본변동표 앞에 — 사용자 선호.)
    pub fn order(&self) -> u8 {
        match self {
            Statement::Bs => 0,
            Statement::Is => 1,
            Statement::Cis => 2,
            Statement::Cf => 3,
            Statement::Sce => 4,
        }
    }

    /// 유량(flow) 제표 여부 — 손익·포괄손익·현금흐름은 기간 누적값이 의미.
    /// (재무상태표는 시점 잔액=stock, 자본변동표는 특수.) 분기/반기 누적값 선택에 사용.
    pub fn is_flow(&self) -> bool {
        matches!(self, Statement::Is | Statement::Cis | Statement::Cf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_sj_div_maps_known_and_rejects_unknown() {
        assert_eq!(Statement::from_sj_div("BS"), Some(Statement::Bs));
        assert_eq!(Statement::from_sj_div("CIS"), Some(Statement::Cis));
        assert_eq!(Statement::from_sj_div("SCE"), Some(Statement::Sce));
        assert_eq!(Statement::from_sj_div("ZZ"), None);
        assert_eq!(Statement::from_sj_div(""), None);
    }

    #[test]
    fn names_are_korean_standard() {
        assert_eq!(Statement::Bs.name(), "재무상태표");
        assert_eq!(Statement::Cf.name(), "현금흐름표");
        assert_eq!(Statement::ALL.len(), 5);
    }
}

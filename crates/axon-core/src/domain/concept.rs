//! 표준 XBRL concept_id (OpenDART `account_id`) 상수 — ratio·footing 등이 계정을 잡는 키.
//! `account_nm`(계정명)은 회사·연도마다 제각각이라 신뢰 불가 → 이 concept_id 로 잡는다.
//! ⚠️ 실데이터 검증 필요 — `dart_` 확장 계정(영업이익 등)은 회사·연도별로 다를 수 있다.

pub const ASSETS: &str = "ifrs-full_Assets";
pub const CURRENT_ASSETS: &str = "ifrs-full_CurrentAssets";
pub const NONCURRENT_ASSETS: &str = "ifrs-full_NoncurrentAssets";
pub const LIABILITIES: &str = "ifrs-full_Liabilities";
pub const CURRENT_LIABILITIES: &str = "ifrs-full_CurrentLiabilities";
pub const NONCURRENT_LIABILITIES: &str = "ifrs-full_NoncurrentLiabilities";
pub const EQUITY: &str = "ifrs-full_Equity";
pub const REVENUE: &str = "ifrs-full_Revenue";
pub const PROFIT_LOSS: &str = "ifrs-full_ProfitLoss";
pub const OPERATING_INCOME: &str = "dart_OperatingIncomeLoss";

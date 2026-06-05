//! Exporter — Table 을 포맷별 바이트로 변환하는 trait.

use crate::domain::table::Table;

pub trait Exporter {
    /// 포맷 식별자(확장자 등, 예: "xlsx").
    fn format(&self) -> &str;

    /// Table 을 해당 포맷의 바이트로 직렬화.
    fn export(&self, table: &Table) -> Vec<u8>;
}

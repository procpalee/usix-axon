//! JsonExporter — Table → 보기 좋은 JSON. 프로그램 연동·재가공용.

use crate::domain::table::Table;
use crate::port::exporter::Exporter;

pub struct JsonExporter;

impl Exporter for JsonExporter {
    fn format(&self) -> &str {
        "json"
    }

    fn export(&self, table: &Table) -> Vec<u8> {
        // Table 은 Serialize 파생 → 실패 거의 불가하지만, panic 대신 폴백.
        serde_json::to_vec_pretty(table).unwrap_or_else(|_| b"{}".to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exports_valid_json() {
        let t = Table {
            headers: vec!["계정".into(), "2023".into()],
            rows: vec![vec!["자산총계".into(), "1,000".into()]],
            keys: vec![],
        };
        let bytes = JsonExporter.export(&t);
        let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(v["headers"][0], "계정");
        assert_eq!(v["rows"][0][1], "1,000");
    }
}

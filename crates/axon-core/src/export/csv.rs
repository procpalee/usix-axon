//! CsvExporter — Table → CSV. 엑셀에서 한글 안 깨지게 UTF-8 BOM, RFC 4180 escape.

use crate::domain::table::Table;
use crate::port::exporter::Exporter;

pub struct CsvExporter;

impl Exporter for CsvExporter {
    fn format(&self) -> &str {
        "csv"
    }

    fn export(&self, table: &Table) -> Vec<u8> {
        let mut out = String::from("\u{feff}"); // BOM → 엑셀이 UTF-8로 인식
        out.push_str(&to_line(&table.headers));
        for row in &table.rows {
            out.push_str(&to_line(row));
        }
        out.into_bytes()
    }
}

fn to_line(cells: &[String]) -> String {
    let joined = cells.iter().map(|c| escape(c)).collect::<Vec<_>>().join(",");
    format!("{joined}\r\n") // CRLF → 엑셀 호환
}

/// RFC 4180: 콤마·따옴표·개행이 있으면 따옴표로 감싸고 내부 따옴표는 두 번.
fn escape(s: &str) -> String {
    if s.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bom_and_crlf() {
        let t = Table {
            headers: vec!["a".into(), "b".into()],
            rows: vec![],
            keys: vec![],
        };
        let bytes = CsvExporter.export(&t);
        assert_eq!(&bytes[0..3], &[0xEF, 0xBB, 0xBF]); // UTF-8 BOM
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("a,b\r\n"));
    }

    #[test]
    fn escapes_comma_and_quote() {
        let t = Table {
            headers: vec!["x".into()],
            rows: vec![vec!["a,b".into()], vec!["he\"llo".into()]],
            keys: vec![],
        };
        let s = String::from_utf8(CsvExporter.export(&t)).unwrap();
        assert!(s.contains("\"a,b\"")); // 콤마 → 감싸기
        assert!(s.contains("\"he\"\"llo\"")); // 따옴표 → 두 번 + 감싸기
    }
}

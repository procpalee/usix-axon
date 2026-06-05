//! axon-ledger — 원장/감사 분석 (IDEA/ACL식). 폴라스 위, **데스크톱 전용**.
//! ⚠️ axon-core(wasm 순수)에 절대 import 금지 — 폴라스는 이 crate 에서만 산다.
//! 차용계획 E: 임포트(CSV/xlsx) → footing·중복·갭·벤포드·샘플링·층화 + cmdlog 감사조서.

use axon_core::domain::table::Table;
use calamine::{Data, Reader, Xlsx};
use csv::{ReaderBuilder, Writer};
use polars::prelude::*;
use std::io::Cursor;

// src-tauri(어댑터)가 임포트한 DataFrame 을 상태로 보관할 수 있게 재노출 — 폴라스 경계는 여기.
pub use polars::prelude::DataFrame;

// 전표 구조/금액 가중 고급 op — 기초 스캔(중복·갭·벤포드)과 분리(320줄 규칙·책임 분리).
mod counterpart;
mod sampling;
pub use counterpart::{analyze_counterpart, CounterpartCols};
pub use sampling::monetary_unit_sampling;

/// CSV 바이트 → DataFrame. (헤더 첫 행 가정.)
pub fn import_csv(bytes: Vec<u8>) -> PolarsResult<DataFrame> {
    CsvReadOptions::default()
        .with_has_header(true)
        .into_reader_with_file_handle(Cursor::new(bytes))
        .finish()
}

/// 원시 문자열 그리드 (헤더 추정 전). 프리뷰·임포트 기준(행/열) 선택용. xlsx 는 첫 시트.
/// 실제 회계 자료는 제목·메타 행이 위에 있어 헤더가 N번째 행인 경우가 흔하다.
pub fn read_grid(bytes: Vec<u8>, is_xlsx: bool) -> Result<Vec<Vec<String>>, String> {
    if is_xlsx {
        let mut wb: Xlsx<_> = Xlsx::new(Cursor::new(bytes)).map_err(|e| e.to_string())?;
        let range = wb
            .worksheet_range_at(0)
            .ok_or_else(|| "시트가 없습니다.".to_string())?
            .map_err(|e| e.to_string())?;
        Ok(range
            .rows()
            .map(|r| r.iter().map(cell_to_string).collect())
            .collect())
    } else {
        ReaderBuilder::new()
            .has_headers(false)
            .flexible(true)
            .from_reader(Cursor::new(bytes))
            .records()
            .map(|r| {
                r.map(|rec| rec.iter().map(|s| s.to_string()).collect())
                    .map_err(|e| e.to_string())
            })
            .collect()
    }
}

fn cell_to_string(d: &Data) -> String {
    match d {
        Data::Empty => String::new(),
        Data::String(s) => s.clone(),
        Data::Float(f) if f.fract() == 0.0 => format!("{}", *f as i64),
        Data::Float(f) => f.to_string(),
        Data::Int(i) => i.to_string(),
        Data::Bool(b) => b.to_string(),
        other => other.to_string(),
    }
}

/// 그리드 + 기준 헤더행 + 컬럼선택 → DataFrame. header_row 행을 컬럼명으로, 그 아래를 데이터로.
/// columns=None 이면 전체. (선택 영역을 CSV 로 재직렬화 → 폴라스 타입추론 재사용.)
pub fn import_grid(
    grid: &[Vec<String>],
    header_row: usize,
    columns: Option<&[usize]>,
) -> Result<DataFrame, String> {
    let header = grid
        .get(header_row)
        .ok_or_else(|| "헤더 행 번호가 범위를 벗어났습니다.".to_string())?;
    let cols: Vec<usize> = match columns {
        Some(c) => c.to_vec(),
        None => (0..header.len()).collect(),
    };
    let mut wtr = Writer::from_writer(Vec::new());
    for row in &grid[header_row..] {
        let rec: Vec<&str> = cols
            .iter()
            .map(|&i| row.get(i).map(String::as_str).unwrap_or(""))
            .collect();
        wtr.write_record(&rec).map_err(|e| e.to_string())?;
    }
    let bytes = wtr.into_inner().map_err(|e| e.to_string())?;
    import_csv(bytes).map_err(|e| e.to_string())
}

/// DataFrame → axon-core Table (헤더=컬럼명, 셀=문자열). 표시·export 파이프라인 재사용.
pub fn to_table(df: &DataFrame) -> Table {
    let headers: Vec<String> = df.get_column_names().iter().map(|c| c.to_string()).collect();
    let rows: Vec<Vec<String>> = (0..df.height())
        .map(|i| {
            df.columns()
                .iter()
                .map(|col| col.get(i).map(|av| av.str_value().to_string()).unwrap_or_default())
                .collect()
        })
        .collect();
    Table { headers, rows, keys: Vec::new() }
}

/// 완전 중복 행(모든 열 동일) 추출. 감사: 중복 전표·중복 지급 탐지(첫 IDEA op).
pub fn find_duplicates(df: &DataFrame) -> PolarsResult<DataFrame> {
    let mask = df.is_duplicated()?;
    df.filter(&mask)
}

/// 갭검출 — 정수 시퀀스 컬럼(전표번호 등)의 [min,max] 범위에서 누락된 번호.
/// 감사 핵심: 빠진 전표·송장 번호 = 누락·은폐 의심. (OSS에 없는 자체구현 = 해자.)
pub fn detect_gaps(df: &DataFrame, col: &str) -> PolarsResult<Vec<i64>> {
    use std::collections::BTreeSet;
    let s = df.column(col)?.as_materialized_series().cast(&DataType::Int64)?;
    let present: BTreeSet<i64> = s.i64()?.into_iter().flatten().collect();
    match (present.iter().next(), present.iter().next_back()) {
        (Some(&lo), Some(&hi)) => Ok((lo..=hi).filter(|n| !present.contains(n)).collect()),
        _ => Ok(vec![]),
    }
}

/// 벤포드 법칙 — 금액 컬럼의 선행자릿수(1-9) 실제분포 vs 기대분포(log10(1+1/d)).
/// 부정탐지: 조작된 수치는 벤포드에서 벗어난다. (선행자릿수 카운트라 f64 OK — 금액 합산 아님.)
pub fn benford(df: &DataFrame, col: &str) -> PolarsResult<Table> {
    let s = df.column(col)?.as_materialized_series().cast(&DataType::Float64)?;
    let mut counts = [0u64; 10];
    let mut total = 0u64;
    for v in s.f64()?.into_iter().flatten() {
        let d = leading_digit(v) as usize;
        if (1..=9).contains(&d) {
            counts[d] += 1;
            total += 1;
        }
    }
    let headers = vec![
        "자릿수".to_string(),
        "실제 건수".to_string(),
        "실제 %".to_string(),
        "기대 % (Benford)".to_string(),
    ];
    let rows = (1..=9usize)
        .map(|d| {
            let actual = if total > 0 { counts[d] as f64 / total as f64 * 100.0 } else { 0.0 };
            let expected = (1.0 + 1.0 / d as f64).log10() * 100.0;
            vec![
                d.to_string(),
                counts[d].to_string(),
                format!("{actual:.1}%"),
                format!("{expected:.1}%"),
            ]
        })
        .collect();
    Ok(Table { headers, rows, keys: Vec::new() })
}

/// 선행 유효자릿수 (1-9), 부호 무관. 0·비유한은 0(제외).
fn leading_digit(v: f64) -> u8 {
    let mut x = v.abs();
    if x == 0.0 || !x.is_finite() {
        return 0;
    }
    while x >= 10.0 {
        x /= 10.0;
    }
    while x < 1.0 {
        x *= 10.0;
    }
    x.floor() as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imports_csv_with_header() {
        let df = import_csv(b"a,b\n1,2\n3,4\n".to_vec()).unwrap();
        assert_eq!(df.height(), 2);
        assert_eq!(df.width(), 2);
    }

    #[test]
    fn finds_duplicate_rows() {
        // "1,2" 가 두 번 → 두 행 모두 중복으로.
        let df = import_csv(b"a,b\n1,2\n1,2\n3,4\n".to_vec()).unwrap();
        assert_eq!(find_duplicates(&df).unwrap().height(), 2);
    }

    #[test]
    fn to_table_roundtrips() {
        let t = to_table(&import_csv(b"a,b\n1,2\n".to_vec()).unwrap());
        assert_eq!(t.headers, vec!["a", "b"]);
        assert_eq!(t.rows[0], vec!["1", "2"]);
    }

    #[test]
    fn read_grid_csv_raw_rows() {
        let g = read_grid(b"a,b\n1,2\n3,4\n".to_vec(), false).unwrap();
        assert_eq!(g.len(), 3);
        assert_eq!(g[0], vec!["a", "b"]);
        assert_eq!(g[2], vec!["3", "4"]);
    }

    #[test]
    fn import_grid_picks_header_row_and_columns() {
        // 위 2줄은 제목·메타(잡음), 3번째 행(index 2)이 진짜 헤더.
        let grid = vec![
            vec!["회사 원장".into(), String::new(), String::new()],
            vec!["2024년".into(), String::new(), String::new()],
            vec!["전표".into(), "계정".into(), "금액".into()],
            vec!["1".into(), "현금".into(), "100".into()],
            vec!["2".into(), "매출".into(), "200".into()],
        ];
        // 헤더=2행, 컬럼 0·2만(전표·금액).
        let df = import_grid(&grid, 2, Some(&[0, 2])).unwrap();
        let t = to_table(&df);
        assert_eq!(t.headers, vec!["전표", "금액"]);
        assert_eq!(df.height(), 2);
        assert_eq!(t.rows[0], vec!["1", "100"]);
    }

    #[test]
    fn detects_sequence_gaps() {
        let df = import_csv(b"no\n1\n2\n4\n5\n".to_vec()).unwrap();
        assert_eq!(detect_gaps(&df, "no").unwrap(), vec![3]);
    }

    #[test]
    fn benford_actual_vs_expected() {
        // 선행자릿수 1·2·3 각 1건 → 실제 33.3%, d=1 기대 30.1%.
        let t = benford(&import_csv(b"amt\n100\n200\n3000\n".to_vec()).unwrap(), "amt").unwrap();
        assert_eq!(t.rows.len(), 9);
        assert_eq!(t.rows[0][2], "33.3%"); // d=1 실제
        assert_eq!(t.rows[0][3], "30.1%"); // d=1 기대
    }
}

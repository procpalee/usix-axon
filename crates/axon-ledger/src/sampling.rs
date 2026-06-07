//! MUS(화폐단위표본추출, PPS) DataFrame 어댑터 — 순수 알고리즘은 axon-core 가 단일 소스로 갖는다.
//! 여기서는 금액열을 f64 슬라이스로 뽑아 코어에 넘기고, 돌아온 (인덱스, 유형)으로 원본 행을
//! take + Type/Book_Value/Audit_Value 컬럼을 부착하는 표현 계층만 담당한다.

use axon_core::service::sampling::{self, MusType};
use polars::prelude::*;

/// MUS 표본추출. `amount_col` 의 양수 행이 대상, `pm`=수행중요성, `confidence`=신뢰수준(0~1).
/// 결과는 선택된 원본 행 + `Type`(Key Item|PPS Sample)·`Book_Value`·`Audit_Value` 컬럼.
pub fn monetary_unit_sampling(
    df: &DataFrame,
    amount_col: &str,
    pm: f64,
    seed: u64,
    confidence: f64,
) -> PolarsResult<DataFrame> {
    let series = df
        .column(amount_col)?
        .as_materialized_series()
        .cast(&DataType::Float64)?;
    let ca = series.f64()?;

    // 길이 = df.height() 인 dense 슬라이스. null → NAN (코어 a > 0.0 에서 탈락 = 기존 null 제외).
    let amounts: Vec<f64> = (0..df.height())
        .map(|i| ca.get(i).unwrap_or(f64::NAN))
        .collect();

    let picks = sampling::monetary_unit_sampling(&amounts, pm, seed, confidence);

    // 라벨 문자열은 표현 계층(여기)에서. 코어는 유형 enum 만 돌려준다.
    let rows: Vec<u32> = picks.iter().map(|p| p.index as u32).collect();
    let types: Vec<&str> = picks
        .iter()
        .map(|p| match p.kind {
            MusType::KeyItem => "Key Item",
            MusType::PpsSample => "PPS Sample",
        })
        .collect();

    // 조립: 선택된 원본 행 take + Type/Book_Value/Audit_Value(감사 전엔 장부가=감사가).
    let idx = IdxCa::from_vec("idx".into(), rows);
    let mut out = df.take(&idx)?;
    out.with_column(Series::new("Type".into(), types).into_column())?;
    let mut bv = out
        .column(amount_col)?
        .as_materialized_series()
        .cast(&DataType::Float64)?;
    bv.rename("Book_Value".into());
    out.with_column(bv.clone().into_column())?;
    bv.rename("Audit_Value".into());
    out.with_column(bv.into_column())?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::import_csv;

    #[test]
    fn key_item_above_interval_is_taken() {
        // 95% 신뢰: R=3.0, interval=300/3.0=100. A=1000 > 100 → Key Item.
        let df = import_csv(b"id,amt\nA,1000\nB,10\nC,20\nD,30\n".to_vec()).unwrap();
        let out = monetary_unit_sampling(&df, "amt", 300.0, 42, 0.95).unwrap();
        let types = out.column("Type").unwrap().as_materialized_series();
        assert!(types.iter().any(|v| v.str_value() == "Key Item"));
        assert!(out.column("Book_Value").is_ok());
        assert!(out.column("Audit_Value").is_ok());
    }

    #[test]
    fn same_seed_is_reproducible() {
        let df = import_csv(b"id,amt\nA,50\nB,60\nC,70\nD,80\nE,90\n".to_vec()).unwrap();
        let a = monetary_unit_sampling(&df, "amt", 200.0, 7, 0.95).unwrap();
        let b = monetary_unit_sampling(&df, "amt", 200.0, 7, 0.95).unwrap();
        assert_eq!(a.height(), b.height());
    }

    // r_factor 포아송표 검증은 순수 코어(axon_core::service::sampling)로 이동했다.

    #[test]
    fn ignores_non_positive_amounts() {
        // 음수·0 제외 → C(500) 만 남고 Key Item.
        let df = import_csv(b"id,amt\nA,-100\nB,0\nC,500\n".to_vec()).unwrap();
        let out = monetary_unit_sampling(&df, "amt", 100.0, 1, 0.90).unwrap();
        assert_eq!(out.height(), 1);
    }
}

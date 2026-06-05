//! MUS(화폐단위표본추출, PPS) — interval=PM/R, 고액(key item) 전수 + 잔여 모집단 체계적 추출.
//! R(신뢰계수) = -ln(1 - confidence), 기대오류 0건 기준 포아송. confidence 는 감사인이 지정.
//!
//! 재현성: 랜덤 시작점을 seed 기반 결정론 PRNG(splitmix64)로 뽑는다 — (모집단, PM, seed)가
//! 같으면 항상 같은 표본. 감사조서는 재수행 시 동일 표본이 나와야 하므로 시스템 난수를 쓰지 않는다.

use polars::prelude::*;

/// splitmix64 한 스텝 → [1e-4, interval) 의 시작점. seed 고정 = 표본 재현.
fn deterministic_start(seed: u64, interval: f64) -> f64 {
    let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    let frac = (z >> 11) as f64 / (1u64 << 53) as f64; // [0,1)
    1e-4 + frac * (interval - 1e-4)
}

/// R-factor: -ln(1 - confidence). 기대오류 0건 기준 포아송.
fn r_factor(confidence: f64) -> f64 {
    -(1.0 - confidence.clamp(0.5, 0.999)).ln()
}

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

    // (원본 행 인덱스, 금액) — 양수만.
    let pop: Vec<(usize, f64)> = (0..df.height())
        .filter_map(|i| ca.get(i).filter(|&a| a > 0.0).map(|a| (i, a)))
        .collect();

    let rf = r_factor(confidence);
    let interval = pm / rf;

    // 분리: key item(interval 이상, 전수) vs PPS 모집단(interval 미만).
    let mut rows: Vec<u32> = Vec::new();
    let mut types: Vec<&str> = Vec::new();
    let mut pps_pop: Vec<(usize, f64)> = Vec::new();
    for &(i, a) in &pop {
        if a >= interval {
            rows.push(i as u32);
            types.push("Key Item");
        } else {
            pps_pop.push((i, a));
        }
    }

    // 체계적 추출: 누적합 위에서 start + j*interval 지점이 떨어지는 행을 고른다.
    if !pps_pop.is_empty() {
        let mut cum = Vec::with_capacity(pps_pop.len());
        let mut acc = 0.0_f64;
        for &(_, a) in &pps_pop {
            acc += a;
            cum.push(acc);
        }
        let total = acc;
        let start = deterministic_start(seed, interval);
        let count = ((total - start) / interval).floor().max(0.0) as i64 + 1;
        let mut last = usize::MAX;
        for j in 0..count {
            let point = start + j as f64 * interval;
            // 첫 cum >= point (numpy searchsorted 'left' 과 동일).
            let idx = cum.partition_point(|&c| c < point);
            if idx < pps_pop.len() && idx != last {
                rows.push(pps_pop[idx].0 as u32);
                types.push("PPS Sample");
                last = idx;
            }
        }
    }

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

    #[test]
    fn r_factor_matches_poisson_table() {
        let eps = 0.01;
        assert!((r_factor(0.90) - 2.303).abs() < eps);
        assert!((r_factor(0.95) - 2.996).abs() < eps);
        assert!((r_factor(0.99) - 4.605).abs() < eps);
    }

    #[test]
    fn ignores_non_positive_amounts() {
        // 음수·0 제외 → C(500) 만 남고 Key Item.
        let df = import_csv(b"id,amt\nA,-100\nB,0\nC,500\n".to_vec()).unwrap();
        let out = monetary_unit_sampling(&df, "amt", 100.0, 1, 0.90).unwrap();
        assert_eq!(out.height(), 1);
    }
}

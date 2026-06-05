//! 상대전표 추출 — 타겟 계정이 든 전표를 모아 방향성으로 '진짜' 상대계정을 분류.
//! voucher 단위 over() 집계: TARGET / TRUE_COUNTERPART / SIDE_ENTRY 라벨 + 전표 차대 무결성.
//! 금액은 f64+eps 비교(벤포드 선례) — 정밀 합계검증은 axon-core 의 footing(rust_decimal) 몫.

use polars::prelude::*;

/// 고객 포맷별 실제 컬럼명 → 표준 역할 매핑. (SAP/AIR/WHG 등 컬럼명 차이를 호출부에서 흡수.)
pub struct CounterpartCols<'a> {
    pub account: &'a str,
    pub voucher: &'a str,
    pub debit: &'a str,
    pub credit: &'a str,
}

/// 타겟 계정이 든 전표만 추출하고, 각 행을 방향성 기준으로 분류한다.
///
/// - `TARGET` — 분석 대상 계정 행.
/// - `TRUE_COUNTERPART` — 타겟의 반대 방향 행(타겟이 차변이면 대변 행, 그 역도). '진짜' 상대계정.
/// - `SIDE_ENTRY` — 같은 전표지만 타겟과 같은 방향(3행 이상 복합전표의 부수 항목).
///
/// `integrity_check` 는 전표별 (차변-대변) 합이 ±eps 를 벗어나면 Trial Balance Mismatch.
pub fn analyze_counterpart(
    df: &DataFrame,
    cols: &CounterpartCols<'_>,
    target: &str,
    eps: f64,
) -> PolarsResult<DataFrame> {
    // 금액 컬럼은 CSV 임포트 시 문자열일 수 있어 매번 Float64 캐스팅 후 사용.
    let debit = || col(cols.debit).cast(DataType::Float64).fill_null(lit(0.0));
    let credit = || col(cols.credit).cast(DataType::Float64).fill_null(lit(0.0));

    let lf = df
        .clone()
        .lazy()
        .with_columns([
            (debit() - credit()).alias("_net"),
            col(cols.account).eq(lit(target)).alias("_is_target"),
            // 타겟 행의 차/대 방향(그 외 행은 null).
            when(col(cols.account).eq(lit(target)))
                .then(
                    when(debit().gt(lit(eps)))
                        .then(lit("DR"))
                        .when(credit().gt(lit(eps)))
                        .then(lit("CR"))
                        .otherwise(lit(NULL)),
                )
                .otherwise(lit(NULL))
                .alias("_row_side"),
        ])
        .with_columns([
            // 전표 단위 집계: 타겟 포함 여부 + 전표 전체의 방향 확정.
            col("_is_target")
                .any(true)
                .over([col(cols.voucher)])
                .alias("_has_target"),
            col("_row_side").max().over([col(cols.voucher)]).alias("_target_side"),
        ])
        .with_columns([
            // 방향성 기반 '진짜' 상대계정: 타겟이 차변이면 같은 전표의 대변 행을 잡는다.
            when(col("_is_target"))
                .then(lit("TARGET"))
                .when(
                    col("_target_side")
                        .eq(lit("DR"))
                        .and(credit().gt(lit(eps)))
                        .or(col("_target_side").eq(lit("CR")).and(debit().gt(lit(eps)))),
                )
                .then(lit("TRUE_COUNTERPART"))
                .otherwise(lit("SIDE_ENTRY"))
                .alias("entry_role"),
            // 전표별 차대합 ≈0 검증 — abs feature 없이 ±eps 범위로.
            when(
                col("_net")
                    .sum()
                    .over([col(cols.voucher)])
                    .gt(lit(eps))
                    .or(col("_net").sum().over([col(cols.voucher)]).lt(lit(-eps))),
            )
            .then(lit("Error: Trial Balance Mismatch"))
            .otherwise(lit("Verified"))
            .alias("integrity_check"),
        ])
        .filter(col("_has_target"));

    let mut out = lf.collect()?;
    // 내부 표식 컬럼 정리 — 결과엔 _net·entry_role·integrity_check 만 노출.
    for tmp in ["_is_target", "_row_side", "_has_target", "_target_side"] {
        let _ = out.drop_in_place(tmp);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::import_csv;

    fn cols() -> CounterpartCols<'static> {
        CounterpartCols { account: "account", voucher: "voucher", debit: "debit", credit: "credit" }
    }

    // V1: Cash(차100)/Sales(대100), V2: Equip(차50)/Cash(대50), V3: Other/Misc (Cash 없음)
    fn sample() -> DataFrame {
        import_csv(
            b"voucher,account,debit,credit\n\
              V1,Cash,100,0\nV1,Sales,0,100\n\
              V2,Equip,50,0\nV2,Cash,0,50\n\
              V3,Other,10,0\nV3,Misc,0,10\n"
                .to_vec(),
        )
        .unwrap()
    }

    fn role_count(df: &DataFrame, role: &str) -> usize {
        df.clone()
            .lazy()
            .filter(col("entry_role").eq(lit(role)))
            .collect()
            .unwrap()
            .height()
    }

    #[test]
    fn extracts_only_vouchers_containing_target() {
        let out = analyze_counterpart(&sample(), &cols(), "Cash", 1e-6).unwrap();
        assert_eq!(out.height(), 4); // V1·V2 의 4행, V3 제외
    }

    #[test]
    fn classifies_true_counterpart_by_direction() {
        let out = analyze_counterpart(&sample(), &cols(), "Cash", 1e-6).unwrap();
        assert_eq!(role_count(&out, "TARGET"), 2); // Cash 2행
        assert_eq!(role_count(&out, "TRUE_COUNTERPART"), 2); // Sales(차변=DR), Equip(대변=CR)
    }

    #[test]
    fn flags_trial_balance_mismatch() {
        // V4: 차100 / 대90 → 합 10 ≠ 0
        let df = import_csv(
            b"voucher,account,debit,credit\nV4,Cash,100,0\nV4,Sales,0,90\n".to_vec(),
        )
        .unwrap();
        let out = analyze_counterpart(&df, &cols(), "Cash", 1e-6).unwrap();
        let ic = out.column("integrity_check").unwrap().as_materialized_series();
        assert!(ic.iter().all(|v| v.str_value().contains("Mismatch")));
    }
}

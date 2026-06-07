//! MUS(화폐단위표본추출, PPS) 순수 코어 — polars/DataFrame 없이 금액 슬라이스 위에서 돈다.
//! interval=PM/R, 고액(key item) 전수 + 잔여 모집단 체계적 추출. wasm-ready(§axon-core 불변식).
//!
//! 재현성: 랜덤 시작점을 seed 기반 결정론 PRNG(splitmix64)로 뽑는다 — (모집단, PM, seed)가
//! 같으면 항상 같은 표본. 감사조서는 재수행 시 동일 표본이 나와야 하므로 시스템 난수를 쓰지 않는다.
//!
//! 어댑터(axon-ledger DataFrame · axon-wasm)가 이 함수를 호출하고, 라벨·열 부착 같은
//! 표현은 각자 계층에서 한다 — 코어는 (인덱스, 유형)만 돌려준다.

use serde::{Deserialize, Serialize};

/// 표본 유형 — Key Item(고액 전수) vs PPS(체계적). 라벨 문자열은 어댑터가 붙인다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MusType {
    KeyItem,
    PpsSample,
}

/// 한 건의 선택 결과 — 호출자 `amounts` 슬라이스 기준 원본 인덱스 + 유형.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MusPick {
    pub index: usize,
    pub kind: MusType,
}

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
pub fn r_factor(confidence: f64) -> f64 {
    -(1.0 - confidence.clamp(0.5, 0.999)).ln()
}

/// MUS 표본추출. `amounts[i]` 는 원본 i번째 행의 금액, 양수만 모집단.
/// `pm`=수행중요성, `seed`=재현용 시드, `confidence`=신뢰수준(0~1).
/// 반환: 선택된 (원본 인덱스, 유형) 목록 — Key Item 먼저, 그다음 PPS 표본.
///
/// 금액은 f64 그대로 둔다 — 선행자릿수·PRNG급 연산이지 Money 합산이 아니다(벤포드 선례와 동일).
pub fn monetary_unit_sampling(
    amounts: &[f64],
    pm: f64,
    seed: u64,
    confidence: f64,
) -> Vec<MusPick> {
    // (원본 행 인덱스, 금액) — 양수만. NaN·음수·0 은 a > 0.0 에서 탈락.
    let pop: Vec<(usize, f64)> = amounts
        .iter()
        .enumerate()
        .filter_map(|(i, &a)| (a > 0.0).then_some((i, a)))
        .collect();

    let interval = pm / r_factor(confidence);

    // 분리: key item(interval 이상, 전수) vs PPS 모집단(interval 미만).
    let mut picks: Vec<MusPick> = Vec::new();
    let mut pps_pop: Vec<(usize, f64)> = Vec::new();
    for &(i, a) in &pop {
        if a >= interval {
            picks.push(MusPick { index: i, kind: MusType::KeyItem });
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
                picks.push(MusPick { index: pps_pop[idx].0, kind: MusType::PpsSample });
                last = idx;
            }
        }
    }

    picks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn r_factor_matches_poisson_table() {
        let eps = 0.01;
        assert!((r_factor(0.90) - 2.303).abs() < eps);
        assert!((r_factor(0.95) - 2.996).abs() < eps);
        assert!((r_factor(0.99) - 4.605).abs() < eps);
    }

    #[test]
    fn key_item_above_interval_is_taken() {
        // 95% 신뢰: R=3.0, interval=300/3.0=100. A=1000 > 100 → Key Item(index 0).
        let picks = monetary_unit_sampling(&[1000.0, 10.0, 20.0, 30.0], 300.0, 42, 0.95);
        assert!(picks.iter().any(|p| p.kind == MusType::KeyItem && p.index == 0));
    }

    #[test]
    fn ignores_non_positive_amounts() {
        // 음수·0·NaN 제외 → index 2(500) 만 남고 Key Item.
        let picks = monetary_unit_sampling(&[-100.0, 0.0, 500.0], 100.0, 1, 0.90);
        assert_eq!(picks.len(), 1);
        assert_eq!(picks[0], MusPick { index: 2, kind: MusType::KeyItem });
    }

    #[test]
    fn nan_amount_is_excluded() {
        // null 을 NAN 으로 넘겨도 a > 0.0 에서 탈락 (어댑터 null 처리 규약).
        let picks = monetary_unit_sampling(&[f64::NAN, 500.0], 100.0, 1, 0.90);
        assert_eq!(picks.len(), 1);
        assert_eq!(picks[0].index, 1);
    }

    #[test]
    fn same_seed_is_reproducible() {
        let amts = [50.0, 60.0, 70.0, 80.0, 90.0];
        let a = monetary_unit_sampling(&amts, 200.0, 7, 0.95);
        let b = monetary_unit_sampling(&amts, 200.0, 7, 0.95);
        assert_eq!(a, b); // 인덱스+유형까지 완전 일치
    }

    #[test]
    fn golden_index_and_type_sequence() {
        // 결정론 골든 벡터 — 교차언어(WASM/데스크톱) 검증 기준값.
        // 95% 신뢰: R≈2.996, interval=600/2.996≈200.3.
        // 1000 ≥ interval → Key Item. 나머지(50·150·120·300...) 중 300·1000? 300<200.3? 아니다 300≥200.3 → Key Item.
        let amts = [1000.0, 50.0, 150.0, 120.0, 300.0, 80.0];
        let picks = monetary_unit_sampling(&amts, 600.0, 42, 0.95);
        // Key Item 은 interval(≈200.3) 이상: 1000(idx0), 300(idx4).
        let keys: Vec<usize> = picks
            .iter()
            .filter(|p| p.kind == MusType::KeyItem)
            .map(|p| p.index)
            .collect();
        assert_eq!(keys, vec![0, 4]);
        // 동일 입력 재호출 시 전체 시퀀스 동일.
        assert_eq!(picks, monetary_unit_sampling(&amts, 600.0, 42, 0.95));
    }
}

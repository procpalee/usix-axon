//! axon-wasm — WASM 웹 어댑터. axon-core 의 순수 결정론 코어를 JS(브라우저·Office.js 애드인)로 노출.
//! ⚠️ polars 절대 금지(axon-ledger 와 분리) — 이 어댑터는 axon-core 위에서만 돈다.
//! 현재 노출: MUS 화폐단위표본추출(monetary_unit_sampling).

use axon_core::service::sampling::{self, MusType};
use serde::Serialize;
use wasm_bindgen::prelude::*;

/// JS 로 직렬화될 한 건의 표본 — 원본(데이터행) 인덱스 + 유형 라벨.
#[derive(Serialize)]
struct Pick {
    index: usize,
    kind: &'static str,
}

/// MUS 표본추출 — JS에서 호출. `amounts`=금액(Float64Array, 데이터행 순서),
/// `pm`=수행중요성, `seed`=재현용 시드(BigInt→u64), `confidence`=신뢰수준(0~1).
/// 반환: `[{index, kind:"Key Item"|"PPS Sample"}, ...]` (선택된 데이터행).
///
/// 라벨 문자열은 이 어댑터가 붙인다 — 코어는 유형 enum 만 돌려준다(데스크톱 ledger 와 동일 규약).
#[wasm_bindgen]
pub fn mus_sample(
    amounts: &[f64],
    pm: f64,
    seed: u64,
    confidence: f64,
) -> Result<JsValue, JsValue> {
    let picks: Vec<Pick> = sampling::monetary_unit_sampling(amounts, pm, seed, confidence)
        .into_iter()
        .map(|p| Pick {
            index: p.index,
            kind: match p.kind {
                MusType::KeyItem => "Key Item",
                MusType::PpsSample => "PPS Sample",
            },
        })
        .collect();
    serde_wasm_bindgen::to_value(&picks).map_err(|e| JsValue::from_str(&e.to_string()))
}

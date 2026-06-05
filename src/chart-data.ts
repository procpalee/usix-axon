// 차트 데이터 준비 — Table(문자열 뷰)에서 차트용 파생값 추출. (concept_id 기반은 후속.)
import { parseNum } from "./chart";
import type { Table } from "./types";

// 공통형(common-size) 분모: 제표별 총계행(BS=자산총계, IS=매출) 값(연도순, 과거→현재).
// ⚠️ concept_id 없이 계정명 매칭(v1) — 못 찾으면 그 제표 계정은 공통형에서 "—" 로 빠진다.
// 견고화(자산총계=ifrs-full_Assets·매출=ifrs-full_Revenue)는 차트가 Fact 를 받게 되면.
export function statementTotals(tbl: Table): Map<string, number[]> {
  const base: Record<string, string[]> = {
    재무상태표: ["자산총계"],
    손익계산서: ["매출액", "수익(매출액)", "영업수익", "매출"],
    포괄손익계산서: ["매출액", "수익(매출액)", "영업수익", "매출"],
  };
  const totals = new Map<string, number[]>();
  for (const [stmt, names] of Object.entries(base)) {
    const r = tbl.rows.find((row) => row[0] === stmt && names.includes(row[1]));
    if (r) totals.set(stmt, r.slice(2).map(parseNum).reverse());
  }
  return totals;
}

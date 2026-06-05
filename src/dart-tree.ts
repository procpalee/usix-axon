import { invoke } from "@tauri-apps/api/core";
import type { Table } from "./types";

export const TOTAL_CONCEPTS = new Set([
  "ifrs-full_Assets", "ifrs-full_Liabilities", "ifrs-full_Equity",
  "ifrs-full_EquityAndLiabilities", "ifrs-full_LiabilitiesAndEquity",
]);
export const SUB_CONCEPTS = new Set([
  "ifrs-full_CurrentAssets", "ifrs-full_NoncurrentAssets",
  "ifrs-full_CurrentLiabilities", "ifrs-full_NoncurrentLiabilities",
]);

export function treeDepths(tbl: Table): number[] {
  const out: number[] = [];
  let stmt: string | null = null;
  let inSec = false;
  let inSub = false;
  tbl.rows.forEach((row, idx) => {
    if (row[0] !== stmt) { stmt = row[0]; inSec = false; inSub = false; }
    const cid = tbl.keys?.[idx] ?? "";
    if (TOTAL_CONCEPTS.has(cid)) { out.push(0); inSec = true; inSub = false; }
    else if (SUB_CONCEPTS.has(cid)) { out.push(1); inSub = true; }
    else if (inSub) out.push(2);
    else if (inSec) out.push(1);
    else out.push(0);
  });
  return out;
}

export interface CalcChild {
  concept_id: string;
  order: number;
  weight: number;
}
export interface CalcTree {
  children: Record<string, CalcChild[]>;
}

const calcCache = new Map<string, CalcTree | null>();
const calcPool: CalcTree[] = [];

export async function calcTreeFor(corp: string, year: number): Promise<CalcTree | null> {
  const k = `${corp}:${year}`;
  const hit = calcCache.get(k);
  if (hit !== undefined) return hit;
  let t: CalcTree | null = null;
  try {
    t = await invoke<CalcTree>("load_calc_tree", { corpCode: corp, year });
  } catch {
    t = null;
  }
  calcCache.set(k, t);
  if (t) calcPool.push(t);
  return t;
}

export async function standardTree(): Promise<CalcTree | null> {
  if (!calcPool.length) return null;
  try {
    return await invoke<CalcTree>("consensus_tree", { trees: calcPool });
  } catch {
    return null;
  }
}

const ROOT_PRIORITY = [
  "ifrs-full_Assets", "ifrs-full_Liabilities", "ifrs-full_Equity", "ifrs-full_EquityAndLiabilities",
];

export function calcLayout(tree: CalcTree): Map<string, { pos: number; depth: number }> {
  const asChild = new Set<string>();
  for (const arr of Object.values(tree.children)) for (const c of arr) asChild.add(c.concept_id);
  const roots = Object.keys(tree.children)
    .filter((k) => !asChild.has(k))
    .sort((a, b) => (ROOT_PRIORITY.indexOf(a) + 1 || 999) - (ROOT_PRIORITY.indexOf(b) + 1 || 999));
  const out = new Map<string, { pos: number; depth: number }>();
  let pos = 0;
  const visit = (cid: string, depth: number) => {
    if (out.has(cid)) return;
    out.set(cid, { pos: pos++, depth });
    for (const ch of tree.children[cid] ?? []) visit(ch.concept_id, depth + 1);
  };
  for (const r of roots) visit(r, 0);
  return out;
}

const STMT_KEY: Record<string, string> = {
  재무상태표: "bs", 손익계산서: "is", 포괄손익계산서: "cis", 현금흐름표: "cf", 자본변동표: "sce",
};
const taxoCache = new Map<string, Record<string, string>>();

export async function ifrsRefMap(stmtName: string): Promise<Record<string, string>> {
  const key = STMT_KEY[stmtName];
  if (!key) return {};
  const hit = taxoCache.get(key);
  if (hit) return hit;
  const m: Record<string, string> = {};
  try {
    const tax = await invoke<{ concepts: { concept_id: string; label_en: string; ifrs_ref: string }[] }>(
      "load_taxonomy",
      { statement: key },
    );
    for (const c of tax.concepts) {
      const parts: string[] = [];
      if (c.label_en) parts.push(c.label_en);
      if (c.ifrs_ref) parts.push(`K-IFRS 근거: ${c.ifrs_ref}`);
      if (parts.length) m[c.concept_id] = parts.join(" · ");
    }
  } catch {
    // 택사노미 fetch 실패 — 툴팁 없이 진행
  }
  taxoCache.set(key, m);
  return m;
}

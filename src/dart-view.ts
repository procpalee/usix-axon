import { invoke } from "@tauri-apps/api/core";
import { emitTo, listen } from "@tauri-apps/api/event";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { parseNum, type Series } from "./chart";
import { openResultView } from "./result-view";
import { statementTotals } from "./chart-data";
import { initMenuActions } from "./menu-actions";
import type { Table } from "./types";
import { $, tr } from "./shell";
import {
  TOTAL_CONCEPTS, treeDepths,
  calcTreeFor, standardTree, calcLayout, ifrsRefMap,
} from "./dart-tree";

let currentTable: Table | null = null;
let chartSel: { corp: string; series: Series[] } = { corp: "", series: [] };

async function syncChart(corp: string): Promise<void> {
  const label = `chart-${corp}`;
  try {
    const ex = await WebviewWindow.getByLabel(label);
    if (ex) {
      await emitTo(label, "chart-set", chartSel.series);
      await ex.setFocus();
    } else if (chartSel.series.length > 0) {
      new WebviewWindow(label, { url: "chart.html", title: `그래프 · ${corp}`, width: 860, height: 480 });
    }
  } catch (err) {
    console.error("chart sync 실패:", err);
  }
}

function showError(msg: string): void {
  const p = document.createElement("p");
  p.className = "msg-err";
  p.textContent = msg;
  resultArea.replaceChildren(p);
}

function withDiffColumns(tbl: Table): Table {
  const headers = tbl.headers.slice(0, 2);
  for (let c = 2; c < tbl.headers.length; c++) headers.push(tbl.headers[c], "Δ%");
  const rows = tbl.rows.map((row) => {
    const out = row.slice(0, 2);
    for (let c = 2; c < row.length; c++) {
      out.push(row[c]);
      const cur = parseNum(row[c]);
      const prev = parseNum(row[c + 1] ?? "");
      out.push(
        !Number.isNaN(cur) && !Number.isNaN(prev) && prev !== 0
          ? `${(((cur - prev) / Math.abs(prev)) * 100).toFixed(1)}%`
          : "",
      );
    }
    return out;
  });
  return { headers, rows, keys: tbl.keys };
}

const resultArea = $<HTMLDivElement>("#result-area");
const corpCode = $<HTMLInputElement>("#corp-code");

async function renderTable(tbl: Table, corp: string, start: number, end: number, basis: string, reprt: string): Promise<void> {
  chartSel = { corp, series: [] };

  let calc = await calcTreeFor(corp, end);
  if (!calc) calc = await standardTree();
  const layout = calc ? calcLayout(calc) : null;
  const fallbackDepths = layout ? null : treeDepths(tbl);
  const tagged = tbl.rows.map((r, i) => ({ r, i, key: tbl.keys?.[i] ?? "" }));
  if (layout) {
    const stmtFirst = new Map<string, number>();
    tbl.rows.forEach((r, i) => {
      if (!stmtFirst.has(r[0])) stmtFirst.set(r[0], i);
    });
    tagged.sort((a, b) => {
      const sa = stmtFirst.get(a.r[0]) ?? 0;
      const sb = stmtFirst.get(b.r[0]) ?? 0;
      if (sa !== sb) return sa - sb;
      const pa = layout.get(a.key)?.pos ?? Infinity;
      const pb = layout.get(b.key)?.pos ?? Infinity;
      return pa !== pb ? pa - pb : a.i - b.i;
    });
  }
  const rows = tagged.map((o) => o.r);
  const keys = tagged.map((o) => o.key);
  const depths = layout
    ? rows.map((_, i) => layout.get(keys[i])?.depth ?? 0)
    : tagged.map((o) => fallbackDepths![o.i]);
  const exportTbl: Table = {
    headers: tbl.headers,
    rows: rows.map((r, i) => [r[0], "  ".repeat(depths[i]) + r[1], ...r.slice(2)]),
    keys,
  };
  currentTable = exportTbl;
  const refMap: Record<string, string> = {};
  for (const s of Array.from(new Set(rows.map((r) => r[0])))) Object.assign(refMap, await ifrsRefMap(s));

  const bar = document.createElement("div");
  bar.className = "result-bar";
  const detach = document.createElement("button");
  detach.textContent = "↗ 새 창";
  detach.onclick = () => {
    new WebviewWindow(`fs-${corp}-${start}-${end}`, {
      url: `statement.html?corp=${corp}&start=${start}&end=${end}`,
      title: `재무제표 · ${corp} · ${start}~${end}`,
      width: 1100,
      height: 720,
    });
  };
  bar.appendChild(detach);
  const exMsg = document.createElement("span");
  exMsg.className = "chart-hint";
  for (const fmt of ["xlsx", "csv", "json"]) {
    const b = document.createElement("button");
    b.textContent = fmt === "xlsx" ? "📊 엑셀" : fmt.toUpperCase();
    b.onclick = async () => {
      exMsg.textContent = "저장 중...";
      try {
        const out = resultArea.classList.contains("show-diff") ? withDiffColumns(exportTbl) : exportTbl;
        const saved = await invoke<string | null>("export_table", { table: out, format: fmt });
        exMsg.textContent = saved ? "✅ 저장됨" : "취소됨";
      } catch (e) {
        exMsg.textContent = `❌ ${e}`;
      }
    };
    bar.appendChild(b);
  }
  const printBtn = document.createElement("button");
  printBtn.textContent = "🖨 PDF/인쇄";
  printBtn.onclick = () => window.print();
  bar.appendChild(printBtn);
  const diffBtn = document.createElement("button");
  diffBtn.textContent = "▲▼ 증감율";
  diffBtn.title = "전년대비 증감율 표시/숨김 (화면·엑셀 공통, 기본 꺼짐)";
  diffBtn.onclick = () => resultArea.classList.toggle("show-diff");
  bar.appendChild(diffBtn);
  const args = { corpCode: corp, start, end, basis, reprt };
  const back = () => void renderTable(tbl, corp, start, end, basis, reprt);
  const ratioBtn = document.createElement("button");
  ratioBtn.textContent = "📈 재무비율";
  ratioBtn.onclick = () => void openResultView(resultArea, "fetch_ratios", args, back);
  bar.appendChild(ratioBtn);
  const footingBtn = document.createElement("button");
  footingBtn.textContent = "🧮 합계검증";
  footingBtn.onclick = () => void openResultView(resultArea, "fetch_footing", args, back);
  bar.appendChild(footingBtn);
  const calcFootingBtn = document.createElement("button");
  calcFootingBtn.textContent = "🔬 정밀검증";
  calcFootingBtn.title = "XBRL 계산식 전체 합계검증 (parent=Σ자식, 상장사)";
  calcFootingBtn.onclick = () =>
    void openResultView(resultArea, "fetch_calc_footing", { corpCode: corp, year: end, basis }, back);
  bar.appendChild(calcFootingBtn);
  bar.appendChild(exMsg);
  const hint = document.createElement("span");
  hint.className = "chart-hint";
  hint.textContent = "행 클릭 → 그래프 (다시 클릭 = 해제)";
  bar.appendChild(hint);

  const years = tbl.headers.slice(2).map((h) => parseInt(h, 10)).reverse();
  const totals = statementTotals(tbl);

  const table = document.createElement("table");
  const headRow = table.createTHead().insertRow();
  for (const h of tbl.headers.slice(1)) {
    const th = document.createElement("th");
    th.textContent = h;
    headRow.appendChild(th);
  }
  const body = table.createTBody();
  const acctTrs: (HTMLTableRowElement | null)[] = new Array(rows.length).fill(null);
  let curStmt: string | null = null;
  let stmtKids: HTMLTableRowElement[] = [];
  rows.forEach((row, idx) => {
    if (row[0] !== curStmt) {
      curStmt = row[0];
      const kids: HTMLTableRowElement[] = [];
      stmtKids = kids;
      const stmt = row[0];
      const gtr = body.insertRow();
      gtr.className = "group-row stmt-row";
      const gc = gtr.insertCell();
      gc.colSpan = tbl.headers.length - 1;
      let open = true;
      const paint = () => (gc.textContent = `${open ? "▾" : "▸"} ${stmt}`);
      paint();
      gtr.onclick = () => {
        open = !open;
        paint();
        kids.forEach((r) => r.classList.toggle("collapsed", !open));
      };
    }

    const d = depths[idx];
    const tr = body.insertRow();
    acctTrs[idx] = tr;
    stmtKids.push(tr);
    tr.classList.add("clickable", `tree-d${d}`);
    if (TOTAL_CONCEPTS.has(keys[idx])) tr.classList.add("tree-total");

    const nameCell = tr.insertCell();
    nameCell.className = "acct-name";
    nameCell.style.paddingLeft = `${8 + d * 16}px`;
    const hasKids = idx + 1 < rows.length && rows[idx + 1][0] === row[0] && depths[idx + 1] > d;
    if (hasKids) {
      const arrow = document.createElement("span");
      arrow.className = "tree-arrow";
      arrow.textContent = "▾";
      let open = true;
      arrow.onclick = (e) => {
        e.stopPropagation();
        open = !open;
        arrow.textContent = open ? "▾" : "▸";
        for (let j = idx + 1; j < acctTrs.length && rows[j][0] === row[0] && depths[j] > d; j++) {
          acctTrs[j]?.classList.toggle("collapsed", !open);
        }
      };
      nameCell.appendChild(arrow);
    }
    const tip = refMap[keys[idx]];
    if (tip) nameCell.title = tip;
    nameCell.appendChild(document.createTextNode(row[1]));

    for (let c = 2; c < row.length; c++) {
      const cell = tr.insertCell();
      cell.textContent = row[c];
      const cur = parseNum(row[c]);
      const prev = parseNum(row[c + 1] ?? "");
      if (!Number.isNaN(cur) && !Number.isNaN(prev) && prev !== 0) {
        const pct = ((cur - prev) / Math.abs(prev)) * 100;
        if (Math.abs(pct) >= 0.05) {
          const span = document.createElement("span");
          span.className = pct >= 0 ? "diff-up" : "diff-down";
          span.textContent = ` ${pct >= 0 ? "▲" : "▼"}${Math.abs(pct).toFixed(1)}%`;
          cell.appendChild(span);
        }
      }
    }

    tr.onclick = () => {
      const account = row[1];
      const i = chartSel.series.findIndex((s) => s.account === account);
      if (i >= 0) {
        chartSel.series.splice(i, 1);
        tr.classList.remove("sel");
      } else {
        chartSel.series.push({ account, years, values: row.slice(2).map(parseNum).reverse(), denoms: totals.get(row[0]) });
        tr.classList.add("sel");
      }
      void syncChart(corp);
    };
  });
  resultArea.replaceChildren(bar, table);
}

export function initDartView(): void {
  initMenuActions(
    resultArea,
    (t) => { currentTable = t; },
    () => { if (currentTable) void invoke("export_table", { table: currentTable, format: "xlsx" }); },
  );

  $("#corp-search-open").addEventListener("click", () => {
    new WebviewWindow("corp-search", {
      url: "search.html",
      title: "회사 검색",
      width: 460,
      height: 560,
    });
  });
  void listen<{ code: string; name: string }>("corp-selected", (e) => {
    corpCode.value = e.payload.code;
    $("#corp-name").textContent = e.payload.name;
  });
  void listen<{ table: Table; corpCode: string; corpName: string; start: number; end: number; basis: string; reprt: string }>(
    "statement-loaded",
    async (e) => {
      const p = e.payload;
      corpCode.value = p.corpCode;
      if (p.corpName) $("#corp-name").textContent = p.corpName;
      $<HTMLInputElement>("#year-start").value = String(p.start);
      $<HTMLInputElement>("#year-end").value = String(p.end);
      await renderTable(p.table, p.corpCode, p.start, p.end, p.basis, p.reprt);
    },
  );
  void listen<string>("chart-ready", (e) => {
    if (e.payload === `chart-${chartSel.corp}`) void emitTo(e.payload, "chart-set", chartSel.series);
  });

  $("#fetch").addEventListener("click", async () => {
    const corp = corpCode.value.trim();
    const start = parseInt($<HTMLInputElement>("#year-start").value.trim(), 10);
    const end = parseInt($<HTMLInputElement>("#year-end").value.trim(), 10);
    if (!corp || !start || !end) {
      showError(tr("needInput"));
      return;
    }
    resultArea.textContent = tr("fetching");
    const basis = $<HTMLSelectElement>("#dart-basis").value;
    const reprt = $<HTMLSelectElement>("#dart-reprt").value;
    try {
      const tbl = await invoke<Table>("fetch_financials_range", { corpCode: corp, start, end, basis, reprt });
      void invoke("set_fin_context", { corpCode: corp, start, end, basis, reprt });
      if (tbl.rows.length === 0) showError(tr("noData"));
      else await renderTable(tbl, corp, start, end, basis, reprt);
    } catch (e) {
      showError(`${tr("fetchFail")}: ${e}`);
    }
  });
}

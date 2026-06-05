import "./styles/index.css";
import { invoke } from "@tauri-apps/api/core";
import { emit, listen } from "@tauri-apps/api/event";

import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import type { ImportResult, Preview } from "./types";

const $ = <T extends HTMLElement>(sel: string): T => document.querySelector<T>(sel)!;

document.body.dataset.theme = localStorage.getItem("axon-theme") ?? "dark";
void listen<string>("theme-changed", (e) => { document.body.dataset.theme = e.payload; });
document.title = "데이터 가져오기";

const root = $<HTMLDivElement>("#wizard-root");

async function loadPreview(): Promise<void> {
  try {
    renderWizard(await invoke<Preview>("ledger_preview"));
  } catch (e) {
    showErr(`프리뷰 실패: ${e}`);
  }
}

void listen("wizard-refresh", () => void loadPreview());
void loadPreview();

function buildGrid(
  rows: string[][],
  startIdx: number,
  showColChecks: boolean,
): { grid: HTMLDivElement; bodyRows: HTMLTableRowElement[]; checks: HTMLInputElement[]; setClick: (h: (globalIdx: number) => void) => void } {
  const ncols = rows.reduce((m, r) => Math.max(m, r.length), 0);
  const grid = document.createElement("div");
  grid.className = "wizard-grid";
  const table = document.createElement("table");
  const chkRow = table.createTHead().insertRow();
  const corner = document.createElement("th");
  corner.textContent = "#";
  chkRow.appendChild(corner);
  const checks: HTMLInputElement[] = [];
  for (let c = 0; c < ncols; c++) {
    const th = document.createElement("th");
    if (showColChecks) {
      const cb = document.createElement("input");
      cb.type = "checkbox";
      cb.checked = true;
      checks.push(cb);
      th.append(cb, document.createTextNode(` 열${c + 1}`));
    } else {
      th.textContent = `열${c + 1}`;
    }
    chkRow.appendChild(th);
  }
  const body = table.createTBody();
  const bodyRows: HTMLTableRowElement[] = [];
  rows.forEach((row, i) => {
    const tr = body.insertRow();
    bodyRows.push(tr);
    tr.insertCell().textContent = String(startIdx + i + 1);
    for (let c = 0; c < ncols; c++) tr.insertCell().textContent = row[c] ?? "";
    tr.style.cursor = "pointer";
  });
  grid.appendChild(table);

  let handler: ((globalIdx: number) => void) | null = null;
  bodyRows.forEach((tr, i) => {
    tr.addEventListener("click", () => handler?.(startIdx + i));
  });

  return { grid, bodyRows, checks, setClick: (h) => { handler = h; } };
}

function renderWizard(pv: Preview): void {
  let headerRow = 0;
  let endRow = pv.total - 1;
  let savedChecks: boolean[] | null = null;

  // ── 1단계: 헤더 행 (상단 프리뷰) ──
  function stepHeader(): void {
    const g = buildGrid(pv.rows, 0, true);
    if (savedChecks) g.checks.forEach((cb, i) => { cb.checked = savedChecks![i] ?? true; });

    const bar = document.createElement("div");
    bar.className = "wizard-toolbar";
    const lbl = document.createElement("label");
    lbl.textContent = "① 헤더 행";
    const hr = document.createElement("input");
    hr.type = "number";
    hr.min = "1";
    hr.max = String(pv.rows.length);
    hr.value = String(headerRow + 1);
    const info = document.createElement("span");
    info.className = "wizard-info";
    info.textContent = `클릭으로 헤더 행 지정 · 전체 ${pv.total}행`;
    const next = document.createElement("button");
    next.textContent = "다음 →";
    bar.append(lbl, hr, info, next);
    root.replaceChildren(bar, g.grid);

    function highlight(): void {
      const idx = parseInt(hr.value, 10) - 1;
      g.bodyRows.forEach((tr, i) => {
        tr.classList.remove("wizard-end-row", "wizard-excluded");
        tr.classList.toggle("wizard-header-row", i === idx);
      });
    }
    highlight();
    hr.addEventListener("input", highlight);
    g.setClick((gi) => { hr.value = String(gi + 1); highlight(); });

    next.onclick = () => {
      headerRow = (parseInt(hr.value, 10) || 1) - 1;
      savedChecks = g.checks.map((cb) => cb.checked);
      stepEnd();
    };
  }

  // ── 2단계: 끝 행 (하단 프리뷰) ──
  function stepEnd(): void {
    const g = buildGrid(pv.tail_rows, pv.tail_start, false);

    const bar = document.createElement("div");
    bar.className = "wizard-toolbar";
    const back = document.createElement("button");
    back.textContent = "← 이전";
    back.className = "wizard-btn-back";
    const lbl = document.createElement("label");
    lbl.textContent = "② 끝 행";
    const er = document.createElement("input");
    er.type = "number";
    er.min = String(headerRow + 2);
    er.max = String(pv.total);
    er.value = String(endRow + 1);
    const info = document.createElement("span");
    info.className = "wizard-info";
    info.textContent = "클릭으로 마지막 데이터 행 지정 (이후 행 제외)";
    const go = document.createElement("button");
    go.textContent = "가져오기";
    bar.append(back, lbl, er, info, go);
    root.replaceChildren(bar, g.grid);
    g.grid.scrollTop = g.grid.scrollHeight;

    function highlight(): void {
      const eIdx = parseInt(er.value, 10) - 1;
      g.bodyRows.forEach((tr, i) => {
        const gi = pv.tail_start + i;
        tr.classList.toggle("wizard-end-row", gi === eIdx);
        tr.classList.toggle("wizard-excluded", gi > eIdx);
      });
    }
    highlight();
    er.addEventListener("input", highlight);
    g.setClick((gi) => {
      if (gi <= headerRow) return;
      er.value = String(gi + 1);
      highlight();
    });

    back.onclick = () => {
      endRow = (parseInt(er.value, 10) || pv.total) - 1;
      stepHeader();
    };

    go.onclick = async () => {
      endRow = (parseInt(er.value, 10) || pv.total) - 1;
      const endArg = endRow < pv.total - 1 ? endRow : null;
      const columns = savedChecks
        ? savedChecks.map((on, i) => (on ? i : -1)).filter((i) => i >= 0)
        : undefined;
      go.disabled = true;
      try {
        const result = await invoke<ImportResult>("ledger_import", { headerRow, endRow: endArg, columns });
        await emit("ledger-imported", result);
        await getCurrentWebviewWindow().close();
      } catch (e) {
        go.disabled = false;
        const err = document.createElement("span");
        err.className = "msg-err";
        err.textContent = ` 실패: ${e}`;
        bar.appendChild(err);
      }
    };
  }

  stepHeader();
}

function showErr(msg: string): void {
  const p = document.createElement("p");
  p.className = "msg-err";
  p.textContent = msg;
  root.replaceChildren(p);
}

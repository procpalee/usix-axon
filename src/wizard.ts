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

function renderWizard(pv: Preview): void {
  const ncols = pv.rows.reduce((m, r) => Math.max(m, r.length), 0);

  // 툴바
  const bar = document.createElement("div");
  bar.className = "wizard-toolbar";
  const lbl = document.createElement("label");
  lbl.textContent = "헤더 행";
  const hr = document.createElement("input");
  hr.type = "number";
  hr.min = "1";
  hr.max = String(pv.rows.length);
  hr.value = "1";
  const info = document.createElement("span");
  info.className = "wizard-info";
  info.textContent = `프리뷰 ${pv.rows.length}행 / 전체 ${pv.total}행`;
  const go = document.createElement("button");
  go.textContent = "가져오기";
  bar.append(lbl, hr, info, go);

  // 그리드
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
    const cb = document.createElement("input");
    cb.type = "checkbox";
    cb.checked = true;
    checks.push(cb);
    th.append(cb, document.createTextNode(` 열${c + 1}`));
    chkRow.appendChild(th);
  }
  const body = table.createTBody();
  const bodyRows: HTMLTableRowElement[] = [];
  pv.rows.forEach((row, i) => {
    const tr = body.insertRow();
    bodyRows.push(tr);
    tr.insertCell().textContent = String(i + 1);
    for (let c = 0; c < ncols; c++) tr.insertCell().textContent = row[c] ?? "";
  });
  grid.appendChild(table);
  root.replaceChildren(bar, grid);

  // 헤더행 하이라이트
  function highlightHeader(): void {
    const idx = parseInt(hr.value, 10) - 1;
    bodyRows.forEach((tr, i) => tr.classList.toggle("wizard-header-row", i === idx));
  }
  highlightHeader();
  hr.addEventListener("input", highlightHeader);

  go.onclick = async () => {
    const headerRow = (parseInt(hr.value, 10) || 1) - 1;
    const columns = checks.map((cb, i) => (cb.checked ? i : -1)).filter((i) => i >= 0);
    go.disabled = true;
    try {
      const result = await invoke<ImportResult>("ledger_import", { headerRow, columns });
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

function showErr(msg: string): void {
  const p = document.createElement("p");
  p.className = "msg-err";
  p.textContent = msg;
  root.replaceChildren(p);
}

// 재무제표 전용 창. URL 파라미터(corp, start, end)로 N년 조회 → 표 렌더.
import "./styles/index.css";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Table } from "./types";
import { renderTable as renderPlainTable } from "./table-renderer";

document.body.dataset.theme = localStorage.getItem("axon-theme") ?? "dark";
void listen<string>("theme-changed", (e) => { document.body.dataset.theme = e.payload; });

const params = new URLSearchParams(location.search);
const corpCode = params.get("corp") ?? "";
const start = parseInt(params.get("start") ?? "", 10);
const end = parseInt(params.get("end") ?? "", 10);

const title = document.querySelector<HTMLHeadingElement>("#fs-title")!;
const result = document.querySelector<HTMLDivElement>("#result-area")!;
title.textContent = `${corpCode} · ${start}~${end}`;

function renderStatementTable(tbl: Table): void {
  result.replaceChildren(renderPlainTable(tbl));
}

async function load(): Promise<void> {
  if (!corpCode || !start || !end) {
    result.textContent = "corp_code / 연도 범위 누락";
    return;
  }
  try {
    const tbl = await invoke<Table>("fetch_financials_range", { corpCode, start, end });
    if (tbl.rows.length === 0) result.textContent = "조회된 데이터가 없습니다.";
    else renderStatementTable(tbl);
  } catch (e) {
    const p = document.createElement("p");
    p.className = "msg-err";
    p.textContent = `조회 실패: ${e}`;
    result.replaceChildren(p);
  }
}

void load();

import type { Table } from "./types";

function fmtCell(v: string): string {
  if (!v) return v;
  const n = Number(v.replace(/,/g, ""));
  if (!Number.isFinite(n)) return v;
  return n.toLocaleString("en-US", { maximumFractionDigits: 10 });
}

export function renderTable(tbl: Table): HTMLTableElement {
  const table = document.createElement("table");
  const headRow = table.createTHead().insertRow();
  for (const h of tbl.headers) {
    const th = document.createElement("th");
    th.textContent = h;
    headRow.appendChild(th);
  }
  const body = table.createTBody();
  for (const row of tbl.rows) {
    const tr = body.insertRow();
    for (const cell of row) tr.insertCell().textContent = fmtCell(cell);
  }
  return table;
}

import type { Table } from "./types";

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
    for (const cell of row) tr.insertCell().textContent = cell;
  }
  return table;
}

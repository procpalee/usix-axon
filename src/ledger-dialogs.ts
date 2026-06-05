import { showModal } from "./modal";

function buildSelect(headers: string[], placeholder: string): HTMLSelectElement {
  const sel = document.createElement("select");
  const def = document.createElement("option");
  def.value = "";
  def.textContent = placeholder;
  sel.appendChild(def);
  for (const h of headers) {
    const o = document.createElement("option");
    o.value = h;
    o.textContent = h;
    sel.appendChild(o);
  }
  return sel;
}

function buildForm(rows: [string, HTMLElement][]): HTMLElement {
  const form = document.createElement("div");
  form.className = "dialog-form";
  for (const [text, input] of rows) {
    const row = document.createElement("div");
    row.className = "dialog-row";
    const lbl = document.createElement("label");
    lbl.textContent = text;
    row.append(lbl, input);
    form.appendChild(row);
  }
  return form;
}

export interface ColumnOp {
  op: string;
  column?: string;
}

export function showColumnOpDialog(
  op: string,
  title: string,
  headers: string[],
  needsColumn: boolean,
): Promise<ColumnOp | null> {
  if (!needsColumn) return Promise.resolve({ op });
  return new Promise((resolve) => {
    const sel = buildSelect(headers, "열 선택…");
    showModal({
      title,
      content: buildForm([["분석 열", sel]]),
      okLabel: "실행",
      cancelLabel: "취소",
      onOk: () => resolve(sel.value ? { op, column: sel.value } : null),
      onCancel: () => resolve(null),
    });
  });
}

export interface MusParams {
  amountCol: string;
  pm: number;
  confidence: number;
}

function buildConfidenceSelect(): HTMLSelectElement {
  const sel = document.createElement("select");
  for (const [label, val] of [["90%", "0.90"], ["95% (권장)", "0.95"], ["99%", "0.99"]]) {
    const o = document.createElement("option");
    o.value = val;
    o.textContent = label;
    if (val === "0.95") o.selected = true;
    sel.appendChild(o);
  }
  return sel;
}

export function showMusDialog(headers: string[]): Promise<MusParams | null> {
  return new Promise((resolve) => {
    const amtSel = buildSelect(headers, "금액열…");
    const pmIn = document.createElement("input");
    pmIn.type = "number";
    pmIn.placeholder = "PM (수행중요성)";
    const confSel = buildConfidenceSelect();
    showModal({
      title: "화폐단위표본추출 (MUS)",
      content: buildForm([
        ["금액열", amtSel],
        ["수행중요성 (PM)", pmIn],
        ["신뢰수준", confSel],
      ]),
      okLabel: "추출",
      cancelLabel: "취소",
      onOk: () => {
        const pm = parseFloat(pmIn.value);
        if (!amtSel.value || !Number.isFinite(pm) || pm <= 0) { resolve(null); return; }
        resolve({ amountCol: amtSel.value, pm, confidence: parseFloat(confSel.value) });
      },
      onCancel: () => resolve(null),
    });
  });
}

export interface CounterpartParams {
  target: string;
  voucherCol: string;
  accountCol: string;
  debitCol: string;
  creditCol: string;
}

export function showCounterpartDialog(headers: string[]): Promise<CounterpartParams | null> {
  return new Promise((resolve) => {
    const tgt = document.createElement("input");
    tgt.type = "text";
    tgt.placeholder = "타겟 계정명";
    const vch = buildSelect(headers, "전표번호열…");
    const acc = buildSelect(headers, "계정명열…");
    const dr = buildSelect(headers, "차변열…");
    const cr = buildSelect(headers, "대변열…");
    showModal({
      title: "상대전표 추출",
      content: buildForm([
        ["타겟 계정", tgt], ["전표번호열", vch], ["계정명열", acc], ["차변열", dr], ["대변열", cr],
      ]),
      okLabel: "추출",
      cancelLabel: "취소",
      onOk: () => {
        if (!tgt.value.trim() || !vch.value || !acc.value || !dr.value || !cr.value) {
          resolve(null);
          return;
        }
        resolve({
          target: tgt.value.trim(),
          voucherCol: vch.value,
          accountCol: acc.value,
          debitCol: dr.value,
          creditCol: cr.value,
        });
      },
      onCancel: () => resolve(null),
    });
  });
}

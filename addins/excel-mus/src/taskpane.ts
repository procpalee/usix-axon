// axon MUS 작업창 — 선택/사용 범위를 읽어 WASM 코어로 표본추출 후 새 시트에 출력.
// 결정론·로컬: 모든 수치 계산은 axon-core(WASM)에서 끝난다. 데이터는 기기를 떠나지 않는다.

import init, { mus_sample } from "./wasm-pkg/axon_wasm.js";

interface Pick {
  index: number;
  kind: string;
}

const $ = <T extends HTMLElement>(id: string): T => document.getElementById(id) as T;

let wasmReady = false;

Office.onReady(async (info) => {
  if (info.host !== Office.HostType.Excel) {
    setStatus("이 애드인은 Excel 전용입니다.", true);
    return;
  }
  await init(); // wasm-pkg(axon_wasm_bg.wasm) 로드 — 1회.
  wasmReady = true;

  $("refresh").addEventListener("click", () => void refreshColumns());
  $("run").addEventListener("click", () => void run());
  attachPmFormatting($<HTMLInputElement>("pm"));
  void refreshColumns();
});

function setStatus(msg: string, isError = false): void {
  const el = $("status");
  el.textContent = msg;
  el.className = "status " + (isError ? "err" : "ok");
}

// PM 입력 천단위 콤마 포맷 — 데스크톱 ledger-dialogs.ts 와 동일 동작.
function attachPmFormatting(pm: HTMLInputElement): void {
  pm.addEventListener("input", () => {
    const raw = pm.value.replace(/,/g, "");
    const n = Number(raw);
    if (raw && Number.isFinite(n)) {
      const pos = pm.selectionStart ?? pm.value.length;
      const lenBefore = pm.value.length;
      pm.value = n.toLocaleString("en-US", { maximumFractionDigits: 10 });
      const delta = pm.value.length - lenBefore;
      pm.setSelectionRange(pos + delta, pos + delta);
    }
  });
}

// 분석 대상 범위 = 사용자가 1셀보다 크게 선택했으면 그 선택, 아니면 활성 시트 사용 영역.
async function loadRange(
  ctx: Excel.RequestContext,
): Promise<{ header: string[]; dataRows: unknown[][] }> {
  const selected = ctx.workbook.getSelectedRange();
  selected.load(["values", "rowCount", "columnCount"]);
  await ctx.sync();

  let values = selected.values;
  if (selected.rowCount < 2 || selected.columnCount < 1) {
    // 단일 셀/빈 선택 → 사용 영역 전체.
    const used = ctx.workbook.worksheets.getActiveWorksheet().getUsedRange();
    used.load("values");
    await ctx.sync();
    values = used.values;
  }
  if (!values || values.length < 2) {
    throw new Error("헤더행 + 데이터행이 최소 2행 필요합니다.");
  }
  const header = values[0].map((v) => String(v));
  const dataRows = values.slice(1);
  return { header, dataRows };
}

async function refreshColumns(): Promise<void> {
  try {
    await Excel.run(async (ctx) => {
      const { header } = await loadRange(ctx);
      const sel = $<HTMLSelectElement>("amount-col");
      sel.innerHTML = "";
      const def = document.createElement("option");
      def.value = "";
      def.textContent = "금액열…";
      sel.appendChild(def);
      for (const h of header) {
        const o = document.createElement("option");
        o.value = h;
        o.textContent = h;
        sel.appendChild(o);
      }
      $<HTMLButtonElement>("run").disabled = !wasmReady;
      setStatus(`열 ${header.length}개 로드됨. 금액열을 선택하세요.`);
    });
  } catch (e) {
    setStatus(String(e instanceof Error ? e.message : e), true);
  }
}

async function run(): Promise<void> {
  const amountCol = $<HTMLSelectElement>("amount-col").value;
  const pm = parseFloat($<HTMLInputElement>("pm").value.replace(/,/g, ""));
  const confidence = parseFloat($<HTMLSelectElement>("confidence").value);
  const seed = BigInt($<HTMLInputElement>("seed").value || "0");

  if (!amountCol) return setStatus("금액열을 선택하세요.", true);
  if (!Number.isFinite(pm) || pm <= 0) return setStatus("PM 을 양수로 입력하세요.", true);

  try {
    await Excel.run(async (ctx) => {
      const { header, dataRows } = await loadRange(ctx);
      const colIdx = header.indexOf(amountCol);
      if (colIdx < 0) throw new Error(`금액열 '${amountCol}'을 찾을 수 없습니다. 새로고침 후 다시.`);

      // 데이터행 → f64. 숫자 아니면(빈칸·문자·텍스트숫자 실패) NaN → 코어 a>0 에서 탈락.
      const amounts = Float64Array.from(dataRows, (r) => {
        const v = r[colIdx];
        if (typeof v === "number") return v;
        const n = Number(String(v).replace(/,/g, ""));
        return Number.isFinite(n) ? n : NaN;
      });

      const picks = mus_sample(amounts, pm, seed, confidence) as Pick[];
      if (picks.length === 0) {
        setStatus("선택된 표본이 없습니다 — PM/금액열을 확인하세요.", true);
        return;
      }

      // 출력 2D: 원본 헤더 + Type/Book_Value/Audit_Value, 그 아래 선택된 데이터행.
      const outHeader = [...header, "Type", "Book_Value", "Audit_Value"];
      const out: unknown[][] = [outHeader];
      for (const p of picks) {
        const src = dataRows[p.index]; // p.index 는 데이터행(헤더 제외) 기준.
        const bv = typeof src[colIdx] === "number" ? src[colIdx] : "";
        out.push([...src, p.kind, bv, bv]); // 감사 전 Book=Audit.
      }

      // 시드를 시트명에 박아 워크페이퍼가 재현 가능하게 (데스크톱 탭 라벨과 동일 취지).
      const name = `MUS ${Math.round(confidence * 100)}% s${seed}`.slice(0, 31);
      const sheet = ctx.workbook.worksheets.add(name);
      sheet.getRangeByIndexes(0, 0, out.length, outHeader.length).values = out;
      sheet.activate();
      await ctx.sync();

      const keyN = picks.filter((p) => p.kind === "Key Item").length;
      setStatus(`표본 ${picks.length}건 (Key Item ${keyN} · PPS ${picks.length - keyN}) → '${name}' 시트.`);
    });
  } catch (e) {
    setStatus(String(e instanceof Error ? e.message : e), true);
  }
}

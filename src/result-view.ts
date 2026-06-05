// 결과 표 뷰 — 임의 결정론 커맨드(fetch_ratios·fetch_footing 등)의 Table 결과를 렌더.
// 읽기 전용 + 재무제표 토글 + 내보내기. 비율·합계검증 공통 (fava "범용 결과표" 발상).
import { invoke } from "@tauri-apps/api/core";
import type { Table } from "./types";
import { renderTable } from "./table-renderer";

/// `command` 결과 Table 을 `area` 에 렌더. `back` = 재무제표 표로 돌아가는 콜백.
export async function openResultView(
  area: HTMLElement,
  command: string,
  args: Record<string, unknown>,
  back: () => void,
): Promise<void> {
  area.textContent = "계산 중...";
  let tbl: Table;
  try {
    tbl = await invoke<Table>(command, args);
  } catch (e) {
    area.textContent = `계산 실패: ${e}`;
    return;
  }

  const bar = document.createElement("div");
  bar.className = "result-bar";
  const backBtn = document.createElement("button");
  backBtn.textContent = "← 재무제표";
  backBtn.onclick = back;
  bar.appendChild(backBtn);

  const exMsg = document.createElement("span");
  exMsg.className = "chart-hint";
  for (const fmt of ["xlsx", "csv", "json"]) {
    const b = document.createElement("button");
    b.textContent = fmt === "xlsx" ? "📊 엑셀" : fmt.toUpperCase();
    b.onclick = async () => {
      exMsg.textContent = "저장 중...";
      try {
        const saved = await invoke<string | null>("export_table", { table: tbl, format: fmt });
        exMsg.textContent = saved ? "✅ 저장됨" : "취소됨";
      } catch (e) {
        exMsg.textContent = `❌ ${e}`;
      }
    };
    bar.appendChild(b);
  }
  bar.appendChild(exMsg);

  area.replaceChildren(bar, renderTable(tbl));
}

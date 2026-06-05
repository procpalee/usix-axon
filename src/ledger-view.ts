import { invoke } from "@tauri-apps/api/core";
import { emitTo, listen } from "@tauri-apps/api/event";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import type { Table, ImportResult } from "./types";
import { initProjectTree } from "./project-tree";
import { initTabbedGrid } from "./tabbed-grid";
import { showColumnOpDialog, showMusDialog, showCounterpartDialog } from "./ledger-dialogs";

const $ = <T extends HTMLElement>(sel: string): T => document.querySelector<T>(sel)!;

type Meta = [string, string][];
const datasetMeta = new Map<string, Meta>();

export function initLedgerView(): void {
  const treeEl = $<HTMLElement>("#dataset-tree");
  const tabBar = $<HTMLElement>("#ledger-tabs");
  const gridArea = $<HTMLElement>("#ledger-grid");
  const exportSel = $<HTMLSelectElement>("#tb-export");

  const tabs = initTabbedGrid(tabBar, gridArea);
  const tree = initProjectTree(treeEl, {
    onSelect: (id, name) => void openDataset(id, name),
    onDelete: (id) => void deleteDataset(id),
  });

  // toolbar
  $("#tb-open").addEventListener("click", () => void invoke("ledger_open"));
  $("#tb-dup").addEventListener("click", () => void runColumnOp("duplicates", "중복 검출", false));
  $("#tb-gap").addEventListener("click", () => void runColumnOp("gaps", "갭 검출", true));
  $("#tb-benford").addEventListener("click", () => void runColumnOp("benford", "벤포드", true));
  $("#tb-mus").addEventListener("click", () => void runMus());
  $("#tb-counter").addEventListener("click", () => void runCounterpart());
  exportSel.addEventListener("change", () => {
    const fmt = exportSel.value;
    if (fmt) { void runExport(fmt); exportSel.value = ""; }
  });

  // wizard flow
  void listen("ledger-preview", () => void openWizard());
  void listen<ImportResult>("ledger-imported", (e) => {
    showLedgerMode();
    tabs.open(e.payload.dataset_id, "임포트", e.payload.table);
    void tree.refresh();
  });
  void listen<string>("ledger-error", (e) => {
    showLedgerMode();
    showErr(gridArea, `원장 임포트 실패: ${e.payload}`);
  });

  async function openDataset(id: string, name: string): Promise<void> {
    if (tabs.has(id)) { tabs.focus(id); return; }
    try {
      const table = await invoke<Table>("ledger_get_dataset", { id });
      tabs.open(id, name, table);
    } catch (e) {
      showErr(gridArea, `${e}`);
    }
  }

  async function deleteDataset(id: string): Promise<void> {
    try {
      await invoke("ledger_delete_dataset", { id });
      tabs.close(id);
      datasetMeta.delete(id);
      await tree.refresh();
    } catch (e) {
      showErr(gridArea, `${e}`);
    }
  }

  async function runColumnOp(op: string, title: string, needsCol: boolean): Promise<void> {
    const headers = tabs.activeHeaders();
    if (!headers.length) { showErr(gridArea, "먼저 파일을 가져오세요."); return; }
    const params = await showColumnOpDialog(op, title, headers, needsCol);
    if (!params) return;
    try {
      const r = await invoke<ImportResult>("ledger_op", { op: params.op, column: params.column });
      tabs.open(r.dataset_id, title, r.table);
      await tree.refresh();
    } catch (e) {
      showErr(gridArea, `${e}`);
    }
  }

  async function runMus(): Promise<void> {
    const headers = tabs.activeHeaders();
    if (!headers.length) { showErr(gridArea, "먼저 파일을 가져오세요."); return; }
    const params = await showMusDialog(headers);
    if (!params) return;
    const seed = crypto.getRandomValues(new Uint32Array(1))[0];
    try {
      const r = await invoke<ImportResult>("ledger_sample", {
        amountCol: params.amountCol, pm: params.pm, seed,
        confidence: params.confidence,
      });
      const pct = Math.round(params.confidence * 100);
      const meta: Meta = [
        ["분석", "화폐단위표본추출 (MUS)"],
        ["금액열", params.amountCol],
        ["수행중요성 (PM)", params.pm.toLocaleString()],
        ["신뢰수준", `${pct}%`],
        ["랜덤 시드", String(seed)],
        ["추출일시", new Date().toLocaleString("ko-KR")],
      ];
      datasetMeta.set(r.dataset_id, meta);
      tabs.open(r.dataset_id, `MUS ${pct}% (시드:${seed})`, r.table);
      await tree.refresh();
    } catch (e) {
      showErr(gridArea, `${e}`);
    }
  }

  async function runCounterpart(): Promise<void> {
    const headers = tabs.activeHeaders();
    if (!headers.length) { showErr(gridArea, "먼저 파일을 가져오세요."); return; }
    const params = await showCounterpartDialog(headers);
    if (!params) return;
    try {
      const r = await invoke<ImportResult>("ledger_counterpart", {
        accountCol: params.accountCol, voucherCol: params.voucherCol,
        debitCol: params.debitCol, creditCol: params.creditCol, target: params.target,
      });
      tabs.open(r.dataset_id, `상대전표(${params.target})`, r.table);
      await tree.refresh();
    } catch (e) {
      showErr(gridArea, `${e}`);
    }
  }

  async function runExport(format: string): Promise<void> {
    const id = tabs.activeId();
    if (!id) { showErr(gridArea, "내보낼 데이터셋이 없습니다."); return; }
    try {
      const table = await invoke<Table>("ledger_get_dataset", { id });
      const meta = datasetMeta.get(id) ?? [];
      await invoke("export_table", { table, format, meta });
    } catch (e) {
      showErr(gridArea, `${e}`);
    }
  }
}

async function openWizard(): Promise<void> {
  const label = "ledger-wizard";
  const ex = await WebviewWindow.getByLabel(label);
  if (ex) {
    await emitTo(label, "wizard-refresh", null);
    await ex.setFocus();
  } else {
    new WebviewWindow(label, { url: "wizard.html", title: "데이터 가져오기", width: 720, height: 560 });
  }
}

function showLedgerMode(): void {
  document.body.dataset.active = "ledger";
  document.querySelectorAll<HTMLElement>(".act-item")
    .forEach((b) => b.classList.toggle("active", b.dataset.view === "ledger"));
  document.querySelectorAll<HTMLElement>(".view")
    .forEach((v) => v.classList.toggle("hidden", v.dataset.view !== "ledger"));
}

function showErr(area: HTMLElement, msg: string): void {
  const p = document.createElement("p");
  p.className = "msg-err";
  p.textContent = msg;
  area.replaceChildren(p);
}

import "./styles/index.css";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { $, tr, getLang, initShell } from "./shell";
import { showModal } from "./modal";
import { initPanel } from "./panel";
import { initDartView } from "./dart-view";
import { initLedgerView } from "./ledger-view";
import { initDaemonSettings, refreshDaemon, refreshMe } from "./daemon-settings";
import { openUrl } from "@tauri-apps/plugin-opener";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { checkForUpdates } from "./updater";
import "./chat";

const LEGAL_BASE = "https://propsol.co.kr";
const TERMS_URL = `${LEGAL_BASE}/terms`;
const PRIVACY_URL = `${LEGAL_BASE}/privacy`;

async function openLegalWindow(label: string, url: string, title: string): Promise<void> {
  const ex = await WebviewWindow.getByLabel(label);
  if (ex) { await ex.setFocus(); return; }
  new WebviewWindow(label, { url, title, width: 800, height: 640 });
}

// ─ 모달 (도움말·약관) ─
void listen("axon://about", () =>
  showModal({ title: tr("aboutTitle"), body: tr("aboutBody"), okLabel: tr("btnClose") }),
);
void listen("axon://terms", () => void openLegalWindow("legal-terms", TERMS_URL, "이용약관"));
void listen("axon://privacy", () => void openLegalWindow("legal-privacy", PRIVACY_URL, "개인정보처리방침"));

function checkTerms(): void {
  if (localStorage.getItem("axon-terms") === "1") return;
  const wrap = document.createElement("div");
  const summary = document.createElement("p");
  summary.textContent = tr("termsSummary");
  const link = document.createElement("a");
  link.textContent = tr("termsLink");
  link.href = "#";
  link.style.display = "block";
  link.style.marginTop = "8px";
  link.onclick = (e) => { e.preventDefault(); void openUrl(TERMS_URL); };
  wrap.append(summary, link);
  showModal({
    title: tr("termsTitle"),
    content: wrap,
    okLabel: tr("btnAccept"),
    onOk: () => localStorage.setItem("axon-terms", "1"),
  });
}

// ─ OpenDART 키 ─
const keyStatus = $<HTMLParagraphElement>("#key-status");
const keyInput = $<HTMLInputElement>("#key-input");
async function refreshKey(): Promise<void> {
  const has = await invoke<boolean>("has_opendart_key");
  keyStatus.textContent = has ? tr("keySet") : tr("keyNone");
  keyStatus.className = "status-line " + (has ? "msg-ok" : "msg-err");
  $("#status-key").textContent = has ? "🔑 OK" : "🔑 —";
}
$("#key-save").addEventListener("click", async () => {
  const key = keyInput.value.trim();
  if (!key) return;
  await invoke("save_opendart_key", { key });
  keyInput.value = "";
  await refreshKey();
});
$("#key-delete").addEventListener("click", async () => {
  await invoke("delete_opendart_key");
  await refreshKey();
});

// ─ init ─
initShell(
  () => void refreshKey(),
  () => { void refreshKey(); void refreshDaemon(); void refreshMe(); },
);
initPanel();
initDartView();
initLedgerView();
initDaemonSettings(() => tr("btnClose"));
checkTerms();
void checkForUpdates();

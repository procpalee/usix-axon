import { invoke } from "@tauri-apps/api/core";
import { $ } from "./shell";

const panel = $<HTMLElement>("#work-panel");
const panelToggle = $<HTMLButtonElement>("#panel-toggle");

function setPanel(open: boolean): void {
  panel.classList.toggle("hidden", !open);
  panelToggle.classList.toggle("active", open);
  localStorage.setItem("axon-panel", open ? "1" : "0");
}

export async function refreshPanelGate(): Promise<void> {
  const hasToken = await invoke<boolean>("has_daemon_token");
  panelToggle.disabled = !hasToken;
  panelToggle.title = hasToken ? "작업 패널 (채팅·조서·분석)" : "usix AI 로그인 후 사용 가능 (설정 ⚙)";
  if (!hasToken) setPanel(false);
  else if (localStorage.getItem("axon-panel") !== "0") setPanel(true);
}

export function initPanel(): void {
  panelToggle.addEventListener("click", () => setPanel(panel.classList.contains("hidden")));
  $("#panel-collapse").addEventListener("click", () => setPanel(false));

  const panelResize = $<HTMLElement>("#panel-resize");
  panelResize.addEventListener("mousedown", (e) => {
    e.preventDefault();
    const startX = e.clientX;
    const startW = panel.getBoundingClientRect().width;
    const onMove = (m: MouseEvent): void => {
      const w = Math.max(280, Math.min(window.innerWidth - 360, startW + (startX - m.clientX)));
      panel.style.setProperty("--panel-w", `${w}px`);
    };
    const onUp = (): void => {
      document.removeEventListener("mousemove", onMove);
      document.removeEventListener("mouseup", onUp);
      localStorage.setItem("axon-panel-w", panel.style.getPropertyValue("--panel-w"));
    };
    document.addEventListener("mousemove", onMove);
    document.addEventListener("mouseup", onUp);
  });

  const savedPanelW = localStorage.getItem("axon-panel-w");
  if (savedPanelW) panel.style.setProperty("--panel-w", savedPanelW);
  void refreshPanelGate();
}

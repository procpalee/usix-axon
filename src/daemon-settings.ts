import { invoke } from "@tauri-apps/api/core";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { $ } from "./shell";
import { showModal } from "./modal";
import { refreshPanelGate } from "./panel";

let features: string[] = [];

function hasFeature(key: string): boolean {
  return features.includes(key);
}
(window as unknown as { hasFeature: (k: string) => boolean }).hasFeature = hasFeature;

function syncWorkpaperGate(): void {
  $("#workpaper-open").hidden = !hasFeature("workpaper");
}

const daemonStatus = $<HTMLParagraphElement>("#daemon-status");

export async function refreshDaemon(): Promise<void> {
  try {
    ($("#daemon-url") as HTMLInputElement).value = await invoke<string>("get_daemon_url");
  } catch {
    /* 미설정 */
  }
  ($("#shell-toggle") as HTMLInputElement).checked = await invoke<boolean>("shell_is_enabled");
  daemonStatus.textContent = (await invoke<boolean>("has_daemon_token")) ? "토큰 설정됨" : "토큰 없음";
}

export async function refreshMe(): Promise<void> {
  const badge = $("#status-plan");
  if (!(await invoke<boolean>("has_daemon_token"))) {
    features = [];
    badge.textContent = "";
    syncWorkpaperGate();
    return;
  }
  try {
    const me = await invoke<{ org: string; plan: string; features: string[] }>("fetch_me");
    features = me.features ?? [];
    badge.textContent = me.plan && me.plan !== "free" ? `○ ${me.plan}` : "";
    badge.title = me.org ? `org: ${me.org}` : "";
  } catch {
    features = [];
    badge.textContent = "";
  }
  syncWorkpaperGate();
}

export function initDaemonSettings(getBtnCloseLabel: () => string): void {
  $("#daemon-url-save").addEventListener("click", async () => {
    try {
      await invoke("save_daemon_url", { url: ($("#daemon-url") as HTMLInputElement).value.trim() });
      daemonStatus.textContent = "주소 저장됨";
    } catch (e) {
      daemonStatus.textContent = `오류: ${e}`;
    }
  });
  $("#daemon-login").addEventListener("click", async () => {
    daemonStatus.textContent = "브라우저에서 로그인 중... (열린 창에서 로그인하세요)";
    try {
      await invoke("daemon_login");
      daemonStatus.textContent = "✅ 로그인됨";
      await refreshDaemon();
      await refreshMe();
      await refreshPanelGate();
    } catch (e) {
      daemonStatus.textContent = `❌ ${e}`;
    }
  });
  $("#daemon-logout").addEventListener("click", async () => {
    await invoke("daemon_logout");
    await refreshDaemon();
    await refreshMe();
    await refreshPanelGate();
  });
  $("#daemon-ping").addEventListener("click", async () => {
    daemonStatus.textContent = "연결 테스트 중...";
    try {
      daemonStatus.textContent = `✅ ${await invoke<string>("ping_daemon")}`;
    } catch (e) {
      daemonStatus.textContent = `❌ ${e}`;
    }
  });
  $("#daemon-clear").addEventListener("click", async () => {
    await invoke("clear_daemon");
    await refreshDaemon();
    await refreshPanelGate();
  });
  $("#shell-toggle").addEventListener("change", async (e) => {
    await invoke("set_shell_enabled", { enabled: (e.target as HTMLInputElement).checked });
  });

  $("#workpaper-open").addEventListener("click", async () => {
    const label = "workpaper";
    const ex = await WebviewWindow.getByLabel(label);
    if (ex) {
      await ex.setFocus();
    } else {
      new WebviewWindow(label, { url: "workpaper.html", title: "조서 자동화", width: 900, height: 640 });
    }
  });
  $("#token-claims").addEventListener("click", async () => {
    try {
      const claims = await invoke<Record<string, unknown>>("daemon_token_claims");
      showModal({ title: "토큰 claim (디버그)", body: JSON.stringify(claims, null, 2), okLabel: getBtnCloseLabel() });
    } catch (e) {
      showModal({ title: "토큰 claim", body: `${e}`, okLabel: getBtnCloseLabel() });
    }
  });

  void refreshMe();
}

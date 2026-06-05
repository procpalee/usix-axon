import { check } from "@tauri-apps/plugin-updater";
import { ask } from "@tauri-apps/plugin-dialog";

export async function checkForUpdates(): Promise<void> {
  try {
    const update = await check();
    if (!update) return;

    const yes = await ask(
      `새 버전 ${update.version}이(가) 있습니다. 지금 업데이트할까요?`,
      { title: "axon 업데이트", kind: "info" },
    );
    if (!yes) return;

    await update.downloadAndInstall();
  } catch {
    // 오프라인이거나 endpoint 미설정 — 무시
  }
}

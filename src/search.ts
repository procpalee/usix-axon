// 회사 검색 전용 창. 검색 → 후보 클릭 → 메인 창으로 corp 전달(emit) → 창 닫기.
import "./styles/index.css";
import { invoke } from "@tauri-apps/api/core";
import { emit, listen } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";

interface Corp {
  code: string;
  name: string;
  stock: string;
}

document.body.dataset.theme = localStorage.getItem("axon-theme") ?? "dark";
void listen<string>("theme-changed", (e) => { document.body.dataset.theme = e.payload; });

const input = document.querySelector<HTMLInputElement>("#corp-search")!;
const results = document.querySelector<HTMLUListElement>("#corp-results")!;
let timer: number | undefined;

input.addEventListener("input", () => {
  clearTimeout(timer);
  const q = input.value.trim();
  if (!q) {
    results.replaceChildren();
    return;
  }
  timer = window.setTimeout(async () => {
    const loading = document.createElement("li");
    loading.textContent = "검색 중... (첫 검색은 회사목록 다운로드로 몇 초)";
    results.replaceChildren(loading);
    try {
      const corps = await invoke<Corp[]>("search_corp", { query: q });
      if (corps.length === 0) {
        loading.textContent = "결과 없음";
        return;
      }
      results.replaceChildren(
        ...corps.map((c) => {
          const li = document.createElement("li");
          li.textContent = c.stock ? `${c.name} (${c.code} · ${c.stock})` : `${c.name} (${c.code})`;
          li.onclick = async () => {
            await emit("corp-selected", { code: c.code, name: c.name });
            await getCurrentWebviewWindow().close();
          };
          return li;
        }),
      );
    } catch (e) {
      loading.className = "msg-err";
      loading.textContent = `검색 실패: ${e}`;
    }
  }, 350);
});

// 조서 자동화 창 — 별도 OS 창. 로그인 + entitlement("workpaper" feature) 게이트로만 진입한다.
// §0: 조서 지능(서술·구성·표본판단)은 daemon 몫(repo 밖). 이 창은 호출·렌더·export 표면일 뿐.
// L1(골격): 분석대상(fin_context) 표시 + 자리. daemon 연결·조서 산출은 L2/L3.
import "./styles/index.css";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface FinContext {
  corp_code: string;
  start: number;
  end: number;
  basis: string | null;
  reprt: string | null;
}

const $ = <T extends HTMLElement>(sel: string): T => document.querySelector<T>(sel)!;

document.body.dataset.theme = localStorage.getItem("axon-theme") ?? "dark";
void listen<string>("theme-changed", (e) => { document.body.dataset.theme = e.payload; });
document.title = "조서 자동화";

void showTarget();

// 화면에 로드된 분석대상(회사·기간)을 헤더에 표시 — 데몬 도구가 검증하는 바로 그 대상.
async function showTarget(): Promise<void> {
  const el = $<HTMLParagraphElement>("#wp-target");
  try {
    const fc = await invoke<FinContext | null>("get_fin_context");
    if (!fc) {
      el.textContent = "분석대상 없음 — DART 분석에서 회사·기간을 먼저 조회하세요.";
      return;
    }
    const basis = fc.basis ? ` · ${fc.basis}` : "";
    el.textContent = `분석대상: ${fc.corp_code} (${fc.start}~${fc.end})${basis}`;
  } catch (e) {
    el.textContent = `분석대상 조회 실패: ${e}`;
  }
}

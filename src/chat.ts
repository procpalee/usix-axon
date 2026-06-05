// daemon 클라이언트 채팅 (① send + 이벤트, ② confirm_exec, ④ stop).
// daemon 응답을 렌더만 한다 — 인지/판단 로직 0 (§0). 표시 전 sanitize 로 ⊙ 상태줄 차단.
import { invoke, Channel } from "@tauri-apps/api/core";

type UiEvent =
  | { kind: "welcome"; sessionId: string }
  | { kind: "chunk"; content: string }
  | { kind: "think"; content: string }
  | { kind: "diagnostic"; content: string }
  | { kind: "tool"; tool: string; ok: boolean; detail: string; preview: string }
  | { kind: "tool_start"; tool: string; detail: string }
  | { kind: "tool_result"; ok: boolean; preview: string }
  | { kind: "permission"; tool: string; reason: string }
  | { kind: "confirm_exec"; callId: string; command: string }
  | { kind: "done" }
  | { kind: "error"; message: string };

let sessionId = localStorage.getItem("usix.sid") ?? crypto.randomUUID();
localStorage.setItem("usix.sid", sessionId);

const log = document.querySelector<HTMLDivElement>("#chat-log")!;
const text = document.querySelector<HTMLTextAreaElement>("#chat-text")!;
const sendBtn = document.querySelector<HTMLButtonElement>("#chat-send")!;
const stopBtn = document.querySelector<HTMLButtonElement>("#chat-stop")!;
const jumpBtn = document.querySelector<HTMLButtonElement>("#chat-jump")!;

// 재진입 잠금 — 턴 진행 중엔 전송을 막고 중지 버튼만 노출(국룰: 스트리밍 중 액션은 중지 하나).
// 잠금이 없으면 두 번째 전송이 데몬 409("처리 중")로 튕기고, 그 에러가 스피너 뒤에 숨어
// "생각 중…"이 영원히 멈춰 보인다. ← 이번 버그의 트리거.
let busy = false;
function setBusy(on: boolean): void {
  busy = on;
  sendBtn.disabled = on;
  sendBtn.style.display = on ? "none" : ""; // 전송↔중지 자리 교체 (inline style = author CSS 이김)
  stopBtn.style.display = on ? "" : "none";
}
setBusy(false);

// 바닥 고정 판정 — 사용자가 위로 올려 과거를 읽는 중이면 스트림이 화면을 끌어내리지 않는다(24px 여유).
function atBottom(): boolean {
  return log.scrollHeight - log.scrollTop - log.clientHeight < 24;
}

// 자동스크롤 게이트(continue 차용) — 매 append 마다 바닥을 재던 샘플링을 scroll 이벤트로 "상태 박제".
// 빠른 스트림 중 사용자가 두 flush 사이에 휠로 올려도 즉시 true 가 박혀, 그 후 어떤 재pin 도 안 끌어내린다.
let userScrolledUp = false;
log.addEventListener("scroll", () => {
  userScrolledUp = !atBottom();
  jumpBtn.hidden = !userScrolledUp; // 위로 올렸을 때만 "맨 아래로" 노출
}, { passive: true });
jumpBtn.addEventListener("click", () => log.scrollTo({ top: log.scrollHeight, behavior: "smooth" }));

// 콘텐츠가 늦게 부푸는 경우(긴 답변 본문·tool/think 버블·textarea 성장으로 log 높이 변동)까지 바닥 유지.
// rAF flush 는 텍스트 변경 시점만 잡으므로 레이아웃 변화는 ResizeObserver 가 보강(continue/assistant-ui 차용).
const pinObserver = new ResizeObserver(() => {
  if (!userScrolledUp) log.scrollTop = log.scrollHeight;
});
pinObserver.observe(log);

// 표시 방어막 — daemon 이 흘리는 ⊙ 상태줄(서버측 내부 지표)은 통째로 버린다.
// 단어가 아니라 마커(⊙)로 거른다: 금지어를 코드에 박지 않기 위해서다.
function sanitize(s: string): string {
  return s
    .split("\n")
    .filter((line) => !line.trimStart().startsWith("⊙"))
    .join("\n");
}

function bubble(cls: string, content = ""): HTMLDivElement {
  const d = document.createElement("div");
  d.className = `bubble ${cls}`;
  d.textContent = content;
  log.appendChild(d);
  pinObserver.observe(d); // 이 버블이 자라면(스트리밍 본문 등) 바닥 유지 트리거
  if (!userScrolledUp) log.scrollTop = log.scrollHeight;
  return d;
}

// 2단계 스피너: "컨텍스트 구성 중"(첫 응답 준비 구간) → 첫 비-welcome 이벤트부터 "생각 중"으로 전환.
let spinner: HTMLDivElement | null = null;
let spinnerLabel = "컨텍스트 구성 중";
let timer: number | undefined;
function startSpinner(): void {
  const t0 = Date.now();
  spinnerLabel = "컨텍스트 구성 중";
  spinner = bubble("think", `💭 ${spinnerLabel}… 0s`);
  timer = window.setInterval(() => {
    if (spinner) spinner.textContent = `💭 ${spinnerLabel}… ${Math.floor((Date.now() - t0) / 1000)}s`;
  }, 1000);
}
function stopSpinner(): void {
  if (timer !== undefined) clearInterval(timer);
  timer = undefined;
  spinner?.remove();
  spinner = null;
}

let answer: HTMLDivElement | null = null;
let buffer = "";
let thinkBubble: HTMLDetailsElement | null = null; // 생각(think_chunk) 누적 — 접이식 <details>, 토큰마다 새로 안 만든다
let thinkBody: HTMLDivElement | null = null; // <details> 안 본문 (여기에 누적 텍스트)
let thinkBuffer = "";
let lastUserContent = ""; // 마지막 사용자 입력 — 에러 후 재시도용

// 청크마다 DOM 을 쓰면 토큰이 쏟아질 때 레이아웃 리플로가 폭주한다. rAF 로 프레임당 한 번만 그린다.
// (Vercel useChat 의 experimental_throttle 과 같은 취지 — 백프레셔 흡수.)
let raf = 0;
function flushRender(): void {
  if (raf) { cancelAnimationFrame(raf); raf = 0; }
  // 생각·답변 모두 누적분을 통째로 정화 → chunk 경계로 ⊙ 줄이 쪼개져도 안전.
  if (thinkBody) thinkBody.textContent = sanitize(thinkBuffer);
  if (answer) answer.textContent = sanitize(buffer);
  if (!userScrolledUp) log.scrollTop = log.scrollHeight;
}
function scheduleRender(): void {
  if (!raf) raf = requestAnimationFrame(flushRender);
}

// 생각(think)은 접이식 <details>에 누적 — 기본 접힘, 요약 클릭 시 펼침. 답변 앞 거대 블록 방지.
function openThink(): void {
  const pin = atBottom();
  const d = document.createElement("details");
  d.className = "bubble think";
  const s = document.createElement("summary");
  s.textContent = "💭 생각 중…";
  s.style.cursor = "pointer";
  thinkBody = document.createElement("div");
  thinkBody.style.marginTop = "4px";
  d.append(s, thinkBody);
  log.appendChild(d);
  pinObserver.observe(d);
  if (pin) log.scrollTop = log.scrollHeight;
  thinkBubble = d;
}
function finalizeThink(): void {
  const s = thinkBubble?.querySelector("summary");
  if (s) s.textContent = "💭 생각"; // 생각 종료 → 라벨 확정(펼치면 전체 보임)
}

// 에러 버블 + 재시도(vercel 차용). daemon 거부(409 "처리 중"·네트워크)는 재시도가 정답인 경우가 많다.
function errorBubble(message: string): void {
  const b = bubble("error", `오류: ${message}`);
  if (!lastUserContent) return;
  const retry = document.createElement("button");
  retry.className = "chat-retry";
  retry.type = "button";
  retry.textContent = "다시 시도";
  retry.addEventListener("click", () => {
    b.remove();
    send(lastUserContent);
  });
  b.appendChild(retry);
}

// 화면에 로드된 재무제표 표 → 탭 구분 텍스트로 직렬화. 접힌 행(display:none)도 DOM 엔 있어 전부 포함.
function serializeStatement(table: HTMLTableElement): string {
  const lines: string[] = [];
  const head = Array.from(table.querySelectorAll("thead th")).map((th) => (th.textContent ?? "").trim());
  if (head.length) lines.push(head.join("\t"));
  for (const tr of Array.from(table.querySelectorAll<HTMLTableRowElement>("tbody tr"))) {
    const cells = Array.from(tr.cells).map((td) => (td.textContent ?? "").trim());
    if (cells.some((c) => c)) lines.push(cells.join("\t"));
  }
  return lines.join("\n");
}

// 작업 컨텍스트 — 회사/기간 + 화면에 로드된 재무제표 실 데이터를 데몬 메시지에 동봉(prepend).
// 데몬이 파일 안 찾고 바로 분석하도록 데이터를 직접 준다. proto 무변경, 표시용 사용자 버블엔 안 들어감.
// (큰 표는 토큰 비용 ↑ — 매 메시지 동봉이라 추후 요약/on-demand 도구로 최적화 여지)
function workspaceContext(): string {
  const parts: string[] = [];
  const corp = document.querySelector<HTMLInputElement>("#corp-code")?.value.trim();
  const name = document.querySelector("#corp-name")?.textContent?.trim();
  const ys = document.querySelector<HTMLInputElement>("#year-start")?.value.trim();
  const ye = document.querySelector<HTMLInputElement>("#year-end")?.value.trim();
  if (corp) parts.push(`회사 ${name || corp}(${corp})`);
  if (ys && ye) parts.push(`기간 ${ys}~${ye}`);
  const head = parts.length ? `[작업 컨텍스트: ${parts.join(" · ")}]\n` : "";
  const table = document.querySelector<HTMLTableElement>("#result-area table");
  if (!table) return head;
  return `${head}[재무제표 데이터 · 탭 구분 · 화면 로드분]\n${serializeStatement(table)}\n\n`;
}

function send(content: string): void {
  if (busy || !content.trim()) return; // 재진입 차단 — 진행 중 재전송이 데몬 409 를 유발했다.
  lastUserContent = content;
  bubble("user", content);
  userScrolledUp = false; // 새 질문 = 다시 바닥을 보고 싶다는 의도(continue) → 자동스크롤 부활
  log.scrollTop = log.scrollHeight; // 내가 보낸 메시지는 항상 바닥으로 따라간다.
  answer = null;
  buffer = "";
  thinkBubble = null;
  thinkBody = null;
  thinkBuffer = "";
  setBusy(true);
  startSpinner();

  const ch = new Channel<UiEvent>();
  ch.onmessage = (e) => {
    // 첫 비-welcome 이벤트(준비 완료 신호)부터 스피너 라벨을 "생각 중"으로.
    if (e.kind !== "welcome" && spinner) spinnerLabel = "생각 중";
    switch (e.kind) {
      case "chunk":
        stopSpinner();
        if (!answer) finalizeThink(); // 첫 답변 토큰 = 생각 끝
        answer ??= bubble("assistant");
        buffer += e.content;
        scheduleRender(); // rAF 스로틀 — sanitize·textContent·스크롤은 flushRender 에서 한 번에
        break;
      case "think":
        stopSpinner();
        if (!thinkBubble) openThink();
        thinkBuffer += e.content;
        scheduleRender();
        break;
      case "diagnostic":
        break; // 서버측 진단 — 무료 클라엔 표시하지 않는다 (§0)
      case "tool_start":
        bubble("tool", `🔧 ${e.tool}${e.detail ? ` ${e.detail}` : ""} …`);
        break;
      case "tool": {
        const head = `🔧 ${e.tool}${e.detail ? ` ${e.detail}` : ""} ${e.ok ? "✓" : "✗"}`;
        const pv = sanitize(e.preview).trimEnd();
        bubble("tool", pv ? `${head}\n${pv}` : head);
        break;
      }
      case "tool_result": {
        const pv = sanitize(e.preview).trimEnd();
        bubble("tool", `${e.ok ? "✓" : "✗"}${pv ? `\n${pv}` : ""}`);
        break;
      }
      case "permission":
        bubble("tool", `🔒 ${e.tool} 거부됨 (${e.reason})`);
        break;
      case "confirm_exec": {
        const ok = confirm(`이 명령을 실행할까요?\n\n${e.command}`);
        void invoke("daemon_approve", { callId: e.callId, approved: ok });
        break;
      }
      case "error":
        flushRender(); // 스트림되던 부분답을 확정한 뒤 에러 표시
        finalizeThink();
        stopSpinner();
        errorBubble(e.message);
        answer = null;
        break;
      case "done":
        flushRender(); // 마지막 청크까지 확정(rAF 비우기 전 answer=null 되면 꼬리 유실)
        finalizeThink();
        stopSpinner();
        answer = null;
        break;
    }
  };
  invoke("daemon_send", { sessionId, content: workspaceContext() + content, thinking: false, channel: ch })
    .catch((err) => {
      // daemon_send Err(reply 409 "처리 중"·키체인·네트워크)는 채널로 error 이벤트가 안 온다.
      // void 로 버리면 스피너가 영원히 돈다 → 여기서 직접 표시 + 재시도 제공.
      stopSpinner();
      errorBubble(String(err));
      answer = null;
    })
    .finally(() => { stopSpinner(); setBusy(false); }); // 백스톱: done 없이 끝나도 스피너·잠금 해제
}

// textarea 자동 높이(LibreChat forceResize 차용) — height='auto' 리셋 후 scrollHeight 적용. cap 은 CSS.
function autosize(): void {
  text.style.height = "auto";
  text.style.height = `${text.scrollHeight}px`;
}
text.addEventListener("input", autosize);

function submit(): void {
  if (busy) return; // 진행 중이면 입력 보존(지우지 않음)
  const v = text.value;
  text.value = "";
  autosize(); // 비운 뒤 높이 리셋 — 안 하면 큰 높이로 고착(LibreChat 함정)
  send(v);
}
sendBtn.addEventListener("click", submit);
// Enter = 전송, Shift+Enter = 줄바꿈. IME 조합 중 Enter(한글 확정)는 전송 안 함.
text.addEventListener("keydown", (e) => {
  if (e.key === "Enter" && !e.shiftKey && !e.isComposing) {
    e.preventDefault();
    submit();
  }
});
stopBtn.addEventListener("click", () => {
  stopSpinner();
  void invoke("daemon_cancel", { sessionId }); // 잠금은 턴이 실제 끝날 때(.finally) 풀린다.
});

// 대화 초기화 — 화면 비우고 새 세션(새 대화) 시작. 진행 중이면 먼저 취소.
const resetBtn = document.querySelector<HTMLButtonElement>("#chat-reset")!;
resetBtn.addEventListener("click", () => {
  if (busy) void invoke("daemon_cancel", { sessionId });
  stopSpinner();
  setBusy(false);
  log.replaceChildren();
  answer = null;
  buffer = "";
  lastUserContent = "";
  userScrolledUp = false;
  jumpBtn.hidden = true;
  text.value = "";
  text.style.height = "auto";
  sessionId = crypto.randomUUID(); // 새 대화 = 새 세션 id
  localStorage.setItem("usix.sid", sessionId);
});

// 그래프 전용 창. 메인이 보내는 '선택 전체'(chart-set)로 매번 갈아끼운다 —
// 증분 add 가 아니라 전체 교체라 이벤트 유실·레이스·선택 해제가 한 번에 해결된다.
// 창이 뜨면 chart-ready 로 자기 label 을 알려, 메인이 현재 선택을 즉시 보내게 한다.
import "./styles/index.css";
import { drawChart, drawSmallMultiples, type Series, type ChartMode } from "./chart";
import { emit, listen } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";

document.body.dataset.theme = localStorage.getItem("axon-theme") ?? "dark";
void listen<string>("theme-changed", (e) => { document.body.dataset.theme = e.payload; });

const root = document.querySelector<HTMLDivElement>("#chart-root")!;
let series: Series[] = [];
let mode: ChartMode = "abs";
let baseIdx = 0;
let split = false; // 분할 보기(small multiples) 토글 — 값 모드와 직교
document.title = "그래프";

void listen<Series[]>("chart-set", (e) => {
  series = e.payload;
  if (baseIdx >= (series[0]?.years.length ?? 1)) baseIdx = 0; // 시리즈 바뀌면 기준 인덱스 보정
  render();
});

// 창 준비 완료 → 메인이 현재 선택 전체를 chart-set 으로 회신.
void emit("chart-ready", getCurrentWebviewWindow().label);
render();

function render(): void {
  const body = split ? drawSmallMultiples(series, mode, baseIdx) : drawChart(series, mode, baseIdx);
  root.replaceChildren(toolbar(), body);
}

function toolbar(): HTMLElement {
  const bar = document.createElement("div");
  bar.className = "chart-toolbar";
  const modes: [ChartMode, string][] = [
    ["abs", "절대값"],
    ["index", "지수 (기준=100)"],
    ["log", "로그"],
    ["common", "공통형 (%)"],
  ];
  for (const [m, label] of modes) {
    const b = document.createElement("button");
    b.textContent = label;
    b.classList.toggle("active", mode === m);
    b.onclick = () => {
      mode = m;
      render();
    };
    bar.appendChild(b);
    // 지수 버튼 바로 옆에 기준 연도 선택 (지수 모드일 때만)
    if (m === "index" && mode === "index") {
      const yrs = series[0]?.years;
      if (yrs && yrs.length) bar.appendChild(baseYearSelect(yrs));
    }
  }
  // 분할 토글 (값 모드와 직교 — 어느 모드든 겹쳐보기↔패널분할)
  const splitBtn = document.createElement("button");
  splitBtn.textContent = "분할";
  splitBtn.classList.toggle("active", split);
  splitBtn.onclick = () => {
    split = !split;
    render();
  };
  bar.appendChild(splitBtn);
  return bar;
}

// 지수 모드 기준 연도 선택 (어느 해를 100 으로 둘지) — 지수 버튼 옆에 표시.
function baseYearSelect(years: number[]): HTMLSelectElement {
  const sel = document.createElement("select");
  years.forEach((y, i) => {
    const o = document.createElement("option");
    o.value = String(i);
    o.textContent = `기준 ${y}`;
    o.selected = i === baseIdx;
    sel.appendChild(o);
  });
  sel.onchange = () => {
    baseIdx = parseInt(sel.value, 10);
    render();
  };
  return sel;
}

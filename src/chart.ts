// 의존 0 SVG 라인 차트. 여러 계정을 다른 색으로 겹쳐 비교 (x축 과거→현재).
// 네 모드: "abs" 절대값 · "index" 지수(기준=100) · "log" 로그 · "common" 공통형(÷제표총계 %).
// 자릿수가 크게 다른 계정(자산총계 vs 현금)도 같은 평면에서 추세 비교.
const NS = "http://www.w3.org/2000/svg";
const PALETTE = ["#0e639c", "#d9534f", "#5cb85c", "#f0ad4e", "#9b59b6", "#17a2b8", "#e83e8c"];

function el<K extends keyof SVGElementTagNameMap>(
  tag: K,
  attrs: Record<string, string | number>,
): SVGElementTagNameMap[K] {
  const e = document.createElementNS(NS, tag);
  for (const [k, v] of Object.entries(attrs)) e.setAttribute(k, String(v));
  return e;
}

// "1,234,567" / "" → number (빈/비수치는 NaN → 차트에서 제외)
export function parseNum(s: string): number {
  if (!s) return NaN;
  const n = Number(s.replace(/,/g, ""));
  return Number.isFinite(n) ? n : NaN;
}

export interface Series {
  account: string;
  years: number[];
  values: number[];
  denoms?: number[]; // 공통형 분모(제표 총계, 연도순) — 있으면 common-size 가능
}

export type ChartMode = "abs" | "index" | "log" | "common";

// 큰 금액 축약 (455905980000000 → "455.9조"). 로그 축 라벨용.
function fmtAbbrev(v: number): string {
  const a = Math.abs(v);
  if (a >= 1e12) return (v / 1e12).toFixed(1).replace(/\.0$/, "") + "조";
  if (a >= 1e8) return Math.round(v / 1e8) + "억";
  if (a >= 1e4) return Math.round(v / 1e4) + "만";
  return String(Math.round(v));
}

// 시리즈를 기준연도(baseIdx) 값 = 100 으로 rebase. 기준값이 0/비수치면 비교 불가 → 전부 NaN(제외).
function rebase(s: Series, baseIdx: number): Series {
  const base = s.values[baseIdx];
  if (!Number.isFinite(base) || base === 0) {
    return { account: s.account, years: s.years, values: s.values.map(() => NaN) };
  }
  return {
    account: s.account,
    years: s.years,
    values: s.values.map((v) => (Number.isFinite(v) ? (v / base) * 100 : NaN)),
  };
}

export function drawChart(list: Series[], mode: ChartMode = "abs", baseIdx = 0): SVGSVGElement {
  const W = 820;
  const H = 360;
  const L = 16;
  const R = 24;
  const T = 32;
  const B = 64;
  const svg = el("svg", { viewBox: `0 0 ${W} ${H}`, class: "chart" });

  // 모드별 값 변환 (modeValues 단일 소스). log 는 비양수 제외 여부를 따로 집계.
  const shown = list.map((s) => ({ account: s.account, years: s.years, values: modeValues(s, mode, baseIdx) }));
  const logDropped = mode === "log" && list.some((s) => s.values.some((v) => Number.isFinite(v) && v <= 0));

  const valid = shown.filter((s) => s.values.some((v) => Number.isFinite(v)));
  if (valid.length === 0) {
    const t = el("text", { x: W / 2, y: H / 2, "text-anchor": "middle", class: "chart-empty" });
    t.textContent = mode === "log" ? "로그 스케일: 표시할 양수 값 없음" : "표시할 수치 없음";
    svg.appendChild(t);
    return svg;
  }

  const years = valid[0].years;
  const all = valid.flatMap((s) => s.values.filter((v) => Number.isFinite(v)));
  const isLog = mode === "log";
  // y 변환: 로그는 log10, 그 외 선형. 기준선 anchor: 절대값=0·지수=100 (로그는 없음).
  const tx = (v: number) => (isLog ? Math.log10(v) : v);
  const anchor = mode === "index" ? 100 : 0;
  const lo = isLog ? Math.min(...all.map(tx)) : Math.min(anchor, ...all);
  const hi = isLog ? Math.max(...all.map(tx)) : Math.max(anchor, ...all);
  const span = hi - lo || 1;
  const n = years.length;
  const xAt = (i: number) => (n === 1 ? W / 2 : L + (i / (n - 1)) * (W - L - R));
  const yAt = (v: number) => T + (1 - (tx(v) - lo) / span) * (H - T - B);

  // 기준선 — 절대값/공통형은 적자 있을 때만(0), 지수는 항상(100). 로그는 기준선 없음.
  if (mode === "index" || ((mode === "abs" || mode === "common") && lo < 0)) {
    svg.appendChild(el("line", { x1: L, y1: yAt(anchor), x2: W - R, y2: yAt(anchor), class: "chart-zero" }));
  }
  // 스케일 감 라벨
  if (mode === "index") {
    const base = el("text", { x: L, y: yAt(100) - 4, class: "chart-vlabel" });
    base.textContent = "100";
    svg.appendChild(base);
    const top = el("text", { x: L, y: T + 8, class: "chart-vlabel" });
    top.textContent = String(Math.round(hi));
    svg.appendChild(top);
  } else if (isLog) {
    const top = el("text", { x: L, y: T + 8, class: "chart-vlabel" });
    top.textContent = fmtAbbrev(Math.max(...all));
    svg.appendChild(top);
    const bot = el("text", { x: L, y: H - B - 2, class: "chart-vlabel" });
    bot.textContent = fmtAbbrev(Math.min(...all));
    svg.appendChild(bot);
    if (logDropped) {
      const note = el("text", { x: W - R, y: T + 8, "text-anchor": "end", class: "chart-xlabel" });
      note.textContent = "적자·0 제외";
      svg.appendChild(note);
    }
  } else if (mode === "common") {
    const top = el("text", { x: L, y: T + 8, class: "chart-vlabel" });
    top.textContent = Math.round(hi) + "%";
    svg.appendChild(top);
  }

  // x축 연도 라벨 (공통)
  years.forEach((y, i) => {
    const xl = el("text", { x: xAt(i), y: H - B + 18, "text-anchor": "middle", class: "chart-xlabel" });
    xl.textContent = String(y);
    svg.appendChild(xl);
  });

  // 시리즈별 라인(다른 색) + 점 + 범례
  valid.forEach((s, si) => {
    const color = PALETTE[si % PALETTE.length];
    const pts = s.years
      .map((_, i) => ({ i, v: s.values[i] }))
      .filter((p) => Number.isFinite(p.v));
    if (pts.length) {
      svg.appendChild(
        el("polyline", {
          points: pts.map((p) => `${xAt(p.i)},${yAt(p.v)}`).join(" "),
          fill: "none",
          stroke: color,
          "stroke-width": 2,
        }),
      );
      pts.forEach((p) => svg.appendChild(el("circle", { cx: xAt(p.i), cy: yAt(p.v), r: 3, fill: color })));
    }
    // 범례 — 3개씩 가로, 넘치면 다음 줄
    const lx = L + (si % 3) * 260;
    const ly = H - 26 + Math.floor(si / 3) * 16;
    svg.appendChild(el("rect", { x: lx, y: ly - 9, width: 12, height: 12, fill: color, rx: 2 }));
    const lt = el("text", { x: lx + 17, y: ly + 1, class: "chart-legend" });
    lt.textContent = s.account;
    svg.appendChild(lt);
  });

  return svg;
}

// 공통형: 각 값 ÷ 제표 총계(denoms) × 100 (%). 분모 없거나 0 이면 비교 불가 → NaN.
function commonValues(s: Series): number[] {
  return s.values.map((v, i) => {
    const d = s.denoms?.[i];
    return Number.isFinite(v) && d !== undefined && d !== 0 ? (v / d) * 100 : NaN;
  });
}

// 같은 시리즈의 모드별 변환값 (index→rebase·log→양수만·common→%·abs→그대로).
function modeValues(s: Series, mode: ChartMode, baseIdx: number): number[] {
  if (mode === "index") return rebase(s, baseIdx).values;
  if (mode === "log") return s.values.map((v) => (Number.isFinite(v) && v > 0 ? v : NaN));
  if (mode === "common") return commonValues(s);
  return s.values;
}

// 미니 패널 1개 — 단일 시리즈, 자기 범위로 자동 스케일 (현재 모드 변환 적용).
function miniPanel(s: Series, mode: ChartMode, baseIdx: number): SVGSVGElement {
  const W = 280;
  const H = 150;
  const L = 8;
  const R = 8;
  const T = 22;
  const B = 22;
  const svg = el("svg", {
    viewBox: `0 0 ${W} ${H}`,
    class: "chart",
    style: "width:280px;max-width:280px;height:auto",
  });
  const title = el("text", { x: W / 2, y: 14, "text-anchor": "middle", class: "chart-legend" });
  title.textContent = s.account;
  svg.appendChild(title);

  const vals = modeValues(s, mode, baseIdx);
  const finite = vals.filter((v) => Number.isFinite(v));
  if (finite.length === 0) {
    const t = el("text", { x: W / 2, y: H / 2, "text-anchor": "middle", class: "chart-empty" });
    t.textContent = "—";
    svg.appendChild(t);
    return svg;
  }

  const isLog = mode === "log";
  const tx = (v: number) => (isLog ? Math.log10(v) : v);
  const lo = Math.min(...finite.map(tx));
  const hi = Math.max(...finite.map(tx));
  const span = hi - lo || 1;
  const n = s.years.length;
  const xAt = (i: number) => (n === 1 ? W / 2 : L + (i / (n - 1)) * (W - L - R));
  const yAt = (v: number) => T + (1 - (tx(v) - lo) / span) * (H - T - B);

  const pts = s.years.map((_, i) => ({ i, v: vals[i] })).filter((p) => Number.isFinite(p.v));
  svg.appendChild(
    el("polyline", {
      points: pts.map((p) => `${xAt(p.i)},${yAt(p.v)}`).join(" "),
      fill: "none",
      stroke: PALETTE[0],
      "stroke-width": 1.5,
    }),
  );
  pts.forEach((p) => svg.appendChild(el("circle", { cx: xAt(p.i), cy: yAt(p.v), r: 2, fill: PALETTE[0] })));

  const maxV = Math.max(...finite);
  const maxL = el("text", { x: L, y: T + 6, class: "chart-vlabel" });
  maxL.textContent = mode === "index" ? String(Math.round(maxV)) : fmtAbbrev(maxV);
  svg.appendChild(maxL);
  [...new Set([0, n - 1])].forEach((i) => {
    const xl = el("text", { x: xAt(i), y: H - 6, "text-anchor": "middle", class: "chart-xlabel" });
    xl.textContent = String(s.years[i]);
    svg.appendChild(xl);
  });
  return svg;
}

// 분할 보기(small multiples) — 계정마다 패널 따로, 각자 스케일. 겹쳐비교 대신 개별 추세 가독.
// 값 모드(절대값/지수/로그)와 직교 — 어느 모드든 분할 가능.
export function drawSmallMultiples(list: Series[], mode: ChartMode = "abs", baseIdx = 0): HTMLElement {
  const grid = document.createElement("div");
  grid.style.cssText = "display:flex;flex-wrap:wrap;gap:8px;justify-content:center;width:100%;overflow:auto;";
  if (list.length === 0) {
    grid.textContent = "표시할 계정 없음";
    return grid;
  }
  for (const s of list) grid.appendChild(miniPanel(s, mode, baseIdx));
  return grid;
}

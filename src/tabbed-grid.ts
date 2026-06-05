import type { Table } from "./types";
import { renderTable } from "./table-renderer";

interface TabEntry {
  id: string;
  name: string;
  table: Table;
}

export interface TabManager {
  open(id: string, name: string, table: Table): void;
  close(id: string): void;
  has(id: string): boolean;
  focus(id: string): void;
  activeId(): string | undefined;
  activeHeaders(): string[];
}

export function initTabbedGrid(tabBar: HTMLElement, gridArea: HTMLElement): TabManager {
  const tabs: TabEntry[] = [];
  let activeIdx = -1;

  function renderTabs(): void {
    tabBar.replaceChildren();
    tabs.forEach((tab, i) => {
      const btn = document.createElement("button");
      btn.className = "grid-tab" + (i === activeIdx ? " active" : "");
      const lbl = document.createElement("span");
      lbl.textContent = tab.name;
      const close = document.createElement("span");
      close.className = "tab-close";
      close.textContent = "✕";
      close.onclick = (e) => { e.stopPropagation(); closeAt(i); };
      btn.append(lbl, close);
      btn.onclick = () => switchTo(i);
      tabBar.appendChild(btn);
    });
  }

  function renderGrid(): void {
    if (activeIdx < 0 || activeIdx >= tabs.length) {
      const p = document.createElement("p");
      p.className = "placeholder";
      p.textContent = "CSV·Excel 파일을 열어 원장 분석을 시작하세요.";
      gridArea.replaceChildren(p);
      return;
    }
    gridArea.replaceChildren(renderTable(tabs[activeIdx].table));
  }

  function switchTo(idx: number): void {
    activeIdx = idx;
    renderTabs();
    renderGrid();
  }

  function closeAt(idx: number): void {
    tabs.splice(idx, 1);
    if (tabs.length === 0) activeIdx = -1;
    else if (activeIdx >= tabs.length) activeIdx = tabs.length - 1;
    else if (activeIdx > idx) activeIdx--;
    renderTabs();
    renderGrid();
  }

  return {
    open(id, name, table) {
      const ex = tabs.findIndex((t) => t.id === id);
      if (ex >= 0) {
        tabs[ex].table = table;
        tabs[ex].name = name;
        switchTo(ex);
      } else {
        tabs.push({ id, name, table });
        switchTo(tabs.length - 1);
      }
    },
    close(id) {
      const idx = tabs.findIndex((t) => t.id === id);
      if (idx >= 0) closeAt(idx);
    },
    has(id) {
      return tabs.some((t) => t.id === id);
    },
    focus(id) {
      const idx = tabs.findIndex((t) => t.id === id);
      if (idx >= 0) switchTo(idx);
    },
    activeId() {
      return activeIdx >= 0 ? tabs[activeIdx]?.id : undefined;
    },
    activeHeaders() {
      return activeIdx >= 0 ? (tabs[activeIdx]?.table.headers ?? []) : [];
    },
  };
}

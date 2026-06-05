import { invoke } from "@tauri-apps/api/core";

interface DatasetNode {
  id: string;
  name: string;
  parent_id: string | null;
  row_count: number;
  col_count: number;
}

export interface TreeCallbacks {
  onSelect: (id: string, name: string) => void;
  onDelete: (id: string) => void;
}

export function initProjectTree(
  container: HTMLElement,
  cb: TreeCallbacks,
): { refresh: () => Promise<void> } {
  async function refresh(): Promise<void> {
    try {
      const nodes = await invoke<DatasetNode[]>("ledger_list_datasets");
      render(container, nodes, cb);
    } catch {
      container.replaceChildren();
    }
  }
  return { refresh };
}

function render(el: HTMLElement, nodes: DatasetNode[], cb: TreeCallbacks): void {
  el.replaceChildren();
  if (nodes.length === 0) {
    const p = document.createElement("p");
    p.className = "tree-empty";
    p.textContent = "데이터셋이 없습니다.";
    el.appendChild(p);
    return;
  }
  const childMap = new Map<string, DatasetNode[]>();
  for (const n of nodes) {
    if (n.parent_id) {
      const arr = childMap.get(n.parent_id) ?? [];
      arr.push(n);
      childMap.set(n.parent_id, arr);
    }
  }
  const ul = document.createElement("ul");
  ul.className = "tree-list";
  for (const n of nodes.filter((n) => !n.parent_id)) {
    ul.appendChild(buildNode(n, childMap, cb, 0));
  }
  el.appendChild(ul);
}

function buildNode(
  node: DatasetNode,
  childMap: Map<string, DatasetNode[]>,
  cb: TreeCallbacks,
  depth: number,
): HTMLLIElement {
  const li = document.createElement("li");
  const row = document.createElement("div");
  row.className = "tree-row";
  row.style.paddingLeft = `${depth * 16 + 4}px`;

  const children = childMap.get(node.id);
  const arrow = document.createElement("span");
  arrow.className = "tree-arrow";

  const label = document.createElement("span");
  label.className = "tree-label";
  label.textContent = node.name;

  const info = document.createElement("span");
  info.className = "tree-info";
  info.textContent = `${node.row_count}×${node.col_count}`;

  const del = document.createElement("button");
  del.className = "tree-del";
  del.textContent = "✕";
  del.title = "삭제";
  del.onclick = (e) => { e.stopPropagation(); cb.onDelete(node.id); };

  row.append(arrow, label, info, del);
  row.onclick = () => cb.onSelect(node.id, node.name);

  if (children) {
    arrow.textContent = "▾";
    const childUl = document.createElement("ul");
    childUl.className = "tree-children";
    for (const c of children) childUl.appendChild(buildNode(c, childMap, cb, depth + 1));
    arrow.onclick = (e) => {
      e.stopPropagation();
      arrow.textContent = childUl.classList.toggle("collapsed") ? "▸" : "▾";
    };
    li.append(row, childUl);
  } else {
    arrow.textContent = " ";
    li.appendChild(row);
  }
  return li;
}

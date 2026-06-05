export interface ModalOptions {
  title: string;
  body?: string;
  content?: HTMLElement;
  okLabel?: string;
  cancelLabel?: string;
  onOk?: () => void;
  onCancel?: () => void;
}

const modal = document.querySelector<HTMLDivElement>("#modal")!;
const titleEl = document.querySelector<HTMLHeadingElement>("#modal-title")!;
const bodyEl = document.querySelector<HTMLParagraphElement>("#modal-body")!;
const actionsEl = modal.querySelector<HTMLDivElement>(".modal-actions")!;

export function showModal(opts: ModalOptions): void {
  titleEl.textContent = opts.title;

  if (opts.content) {
    bodyEl.replaceChildren(opts.content);
  } else {
    bodyEl.textContent = opts.body ?? "";
  }

  actionsEl.replaceChildren();

  if (opts.cancelLabel) {
    const cancel = document.createElement("button");
    cancel.textContent = opts.cancelLabel;
    cancel.style.marginRight = "8px";
    cancel.style.background = "transparent";
    cancel.style.color = "var(--text)";
    cancel.style.border = "1px solid var(--border)";
    cancel.onclick = () => {
      closeModal();
      opts.onCancel?.();
    };
    actionsEl.appendChild(cancel);
  }

  const ok = document.createElement("button");
  ok.textContent = opts.okLabel ?? "확인";
  ok.onclick = () => {
    closeModal();
    opts.onOk?.();
  };
  actionsEl.appendChild(ok);

  modal.classList.remove("hidden");
}

export function closeModal(): void {
  modal.classList.add("hidden");
}

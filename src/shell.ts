import { applyI18n, t, type Lang, type MsgKey } from "./i18n";
import { emit } from "@tauri-apps/api/event";

export const $ = <T extends HTMLElement>(sel: string): T =>
  document.querySelector<T>(sel)!;

let lang: Lang = (localStorage.getItem("axon-lang") as Lang) ?? "ko";
let theme: string = localStorage.getItem("axon-theme") ?? "dark";

export function getLang(): Lang {
  return lang;
}
export function tr(key: MsgKey): string {
  return t(lang, key);
}

export function applyTheme(): void {
  document.body.dataset.theme = theme;
  $("#theme-toggle").textContent = theme === "dark" ? "🌙" : "☀️";
}

export function switchView(view: string): void {
  document.body.dataset.active = view;
  document
    .querySelectorAll<HTMLElement>(".act-item")
    .forEach((b) => b.classList.toggle("active", b.dataset.view === view));
  document
    .querySelectorAll<HTMLElement>(".view")
    .forEach((v) => v.classList.toggle("hidden", v.dataset.view !== view));
}

export function initShell(onLangChange: () => void, onSettingsOpen: () => void): void {
  $("#theme-toggle").addEventListener("click", () => {
    theme = theme === "dark" ? "light" : "dark";
    localStorage.setItem("axon-theme", theme);
    applyTheme();
    void emit("theme-changed", theme);
  });

  const applyLang = (): void => {
    applyI18n(lang);
    document.documentElement.lang = lang;
    ($("#lang-select") as HTMLSelectElement).value = lang;
    onLangChange();
  };
  $("#lang-select").addEventListener("change", (e) => {
    lang = (e.target as HTMLSelectElement).value as Lang;
    localStorage.setItem("axon-lang", lang);
    applyLang();
  });

  document.querySelectorAll<HTMLElement>(".act-item").forEach((b) =>
    b.addEventListener("click", () => {
      if (!b.dataset.view) return;
      switchView(b.dataset.view!);
      if (b.dataset.view === "settings") onSettingsOpen();
    }),
  );

  applyTheme();
  applyLang();
}

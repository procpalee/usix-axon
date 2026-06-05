// UI 문자열 사전 (ko/en). "잘나갈 때" 영어 전환 대비.
// 동적 에러 메시지(OpenDART status 등)의 i18n 은 다음 단계 — 지금은 정적 UI 텍스트.

export type Lang = "ko" | "en";

export const MESSAGES = {
  ko: {
    dart: "DART 분석",
    keySection: "OpenDART 키",
    keyPh: "인증키",
    save: "저장",
    delete: "삭제",
    fsSection: "재무제표 조회",
    corpPh: "corp_code (8자리)",
    yearPh: "연도 (예: 2023)",
    fetch: "조회",
    corpSearchPh: "회사명 검색",
    resultPlaceholder: "왼쪽에서 조회하면 결과가 여기 표시됩니다.",
    keyChecking: "확인 중...",
    keySet: "✅ 키 설정됨",
    keyNone: "⚠️ 키 없음",
    fetching: "조회 중...",
    needInput: "corp_code 와 연도를 입력하세요.",
    fetchFail: "조회 실패",
    noData: "조회된 데이터가 없습니다.",
    aboutTitle: "axon 정보",
    aboutBody:
      "회계 결정론 분석 도구\n\n버전 0.0.0\n라이선스 AGPL-3.0\n데이터 출처: 금융감독원 OpenDART",
    termsTitle: "이용약관",
    termsSummary: "axon 서비스를 이용하시려면 이용약관에 동의해야 합니다.",
    termsLink: "이용약관 전문 보기 →",
    btnAccept: "동의하고 시작",
    btnClose: "닫기",
  },
  en: {
    dart: "DART Analysis",
    keySection: "OpenDART Key",
    keyPh: "API key",
    save: "Save",
    delete: "Delete",
    fsSection: "Financial Statements",
    corpPh: "corp_code (8 digits)",
    yearPh: "Year (e.g. 2023)",
    fetch: "Fetch",
    corpSearchPh: "Search company",
    resultPlaceholder: "Query from the sidebar to see results here.",
    keyChecking: "Checking...",
    keySet: "✅ Key set",
    keyNone: "⚠️ No key",
    fetching: "Fetching...",
    needInput: "Enter corp_code and year.",
    fetchFail: "Fetch failed",
    noData: "No data found.",
    aboutTitle: "About axon",
    aboutBody:
      "Deterministic accounting analysis tool\n\nVersion 0.0.0\nLicense AGPL-3.0\nData source: FSS OpenDART",
    termsTitle: "Terms of Service",
    termsSummary: "You must agree to the Terms of Service to use axon.",
    termsLink: "View full Terms of Service →",
    btnAccept: "Agree & Start",
    btnClose: "Close",
  },
} as const;

export type MsgKey = keyof (typeof MESSAGES)["ko"];

export function t(lang: Lang, key: MsgKey): string {
  return MESSAGES[lang][key];
}

/// data-i18n(텍스트) / data-i18n-ph(placeholder) 속성을 가진 요소에 사전을 적용.
export function applyI18n(lang: Lang): void {
  document.querySelectorAll<HTMLElement>("[data-i18n]").forEach((el) => {
    el.textContent = MESSAGES[lang][el.dataset.i18n as MsgKey];
  });
  document.querySelectorAll<HTMLInputElement>("[data-i18n-ph]").forEach((el) => {
    el.placeholder = MESSAGES[lang][el.dataset.i18nPh as MsgKey];
  });
}

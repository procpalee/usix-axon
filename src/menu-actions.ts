// 앱 메뉴(파일/보기) → 프론트 액션. 백엔드 menu.rs 가 이벤트를 emit, 여기서 처리.
// 새로 만들기(결과 비우기)·저장(현재 표 내보내기). 파일 열기=원장 임포트는 ledger-view 가 처리.
import { listen } from "@tauri-apps/api/event";
import type { Table } from "./types";

/// 메뉴 이벤트를 화면 액션에 연결.
/// area: 결과 표시 영역 · setCurrent: 현재 표 갱신(저장용) · exportCurrent: 현재 표 내보내기.
export function initMenuActions(
  area: HTMLElement,
  setCurrent: (t: Table | null) => void,
  exportCurrent: () => void,
): void {
  // 새로 만들기 → 결과 비우기
  void listen("axon://new", () => {
    area.replaceChildren();
    setCurrent(null);
  });
  // 저장 → 현재 표 내보내기
  void listen("axon://save", () => exportCurrent());
}

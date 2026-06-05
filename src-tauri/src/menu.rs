//! 앱 메뉴바 — 파일·편집·선택영역·보기·도움말. 커스텀 항목은 on_event 에서 처리.

use tauri::menu::{Menu, MenuEvent, SubmenuBuilder};
use tauri::{AppHandle, Emitter, Manager, Runtime};

pub fn build<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let file = SubmenuBuilder::new(app, "파일(F)")
        .text("file.new", "새로 만들기")
        .text("file.open", "열기...")
        .text("file.save", "저장")
        .separator()
        .quit()
        .build()?;

    let edit = SubmenuBuilder::new(app, "편집(E)")
        .undo()
        .redo()
        .separator()
        .cut()
        .copy()
        .paste()
        .build()?;

    let selection = SubmenuBuilder::new(app, "선택 영역(S)")
        .select_all()
        .build()?;

    let view = SubmenuBuilder::new(app, "보기(V)")
        .text("view.reload", "새로고침")
        .build()?;

    let help = SubmenuBuilder::new(app, "도움말(H)")
        .text("help.about", "axon 정보")
        .separator()
        .text("help.terms", "이용약관")
        .text("help.privacy", "개인정보처리방침")
        .build()?;

    Menu::with_items(app, &[&file, &edit, &selection, &view, &help])
}

pub fn on_event<R: Runtime>(app: &AppHandle<R>, event: MenuEvent) {
    // 커스텀 항목 처리 (predefined undo/copy/quit 등은 OS 가 알아서).
    match event.id().as_ref() {
        "help.about" => {
            let _ = app.emit("axon://about", ());
        }
        "help.terms" => {
            let _ = app.emit("axon://terms", ());
        }
        "help.privacy" => {
            let _ = app.emit("axon://privacy", ());
        }
        "view.reload" => {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.reload();
            }
        }
        // "파일 열기" = 원장 CSV 임포트 → ledger-table 이벤트로 프론트 렌더.
        "file.open" => crate::ledger::open_and_import(app),
        // 프론트가 처리: 새로 만들기=결과 비우기, 저장=현재 표 내보내기.
        "file.new" => {
            let _ = app.emit("axon://new", ());
        }
        "file.save" => {
            let _ = app.emit("axon://save", ());
        }
        _ => {}
    }
}

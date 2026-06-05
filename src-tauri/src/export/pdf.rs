//! PDF 내보내기 — 테이블 + 메타 헤더(MUS 시드 등). 시스템 한글 폰트 자동 탐색.

use axon_core::domain::table::Table;
use printpdf::*;
use std::io::Cursor;

const W: f32 = 210.0;
const H: f32 = 297.0;
const M: f32 = 15.0;
const USABLE_W: f32 = W - 2.0 * M;

fn find_korean_font() -> Option<Vec<u8>> {
    let paths = [
        "/usr/share/fonts/gothic-a1/GothicA1-Regular.ttf",
        "/usr/share/fonts/nanum/NanumGothic.ttf",
        "/usr/share/fonts/truetype/nanum/NanumGothic.ttf",
        "/usr/share/fonts/nanum-gothic/NanumGothic-Regular.ttf",
        "/usr/share/fonts/google-noto/NotoSansKR-Regular.ttf",
    ];
    for p in paths {
        if let Ok(bytes) = std::fs::read(p) {
            return Some(bytes);
        }
    }
    #[cfg(target_os = "windows")]
    if let Ok(bytes) = std::fs::read("C:\\Windows\\Fonts\\malgun.ttf") {
        return Some(bytes);
    }
    None
}

pub fn export_pdf(table: &Table, meta: &[(String, String)]) -> Result<Vec<u8>, String> {
    let (doc, page1, layer1) = PdfDocument::new("axon", Mm(W), Mm(H), "L");

    let font = if let Some(bytes) = find_korean_font() {
        doc.add_external_font(Cursor::new(bytes))
            .map_err(|e| format!("폰트 로드 실패: {e}"))?
    } else {
        doc.add_builtin_font(BuiltinFont::Helvetica)
            .map_err(|e| format!("기본 폰트 실패: {e}"))?
    };
    let font_bold = doc
        .add_builtin_font(BuiltinFont::HelveticaBold)
        .map_err(|e| format!("{e}"))?;

    let ncols = table.headers.len().max(1);
    let col_w = USABLE_W / ncols as f32;
    let mut y = H - M;
    let mut layer = doc.get_page(page1).get_layer(layer1);

    // 메타 헤더
    if !meta.is_empty() {
        layer.use_text("axon 분석 결과", 13.0, Mm(M), Mm(y), &font_bold);
        y -= 7.0;
        for (k, v) in meta {
            layer.use_text(&format!("{k}: {v}"), 9.0, Mm(M), Mm(y), &font);
            y -= 4.5;
        }
        y -= 4.0;
    }

    let header_y = y;
    write_headers(&layer, &font_bold, &table.headers, col_w, y);
    y -= 5.0;

    for row in &table.rows {
        if y < M + 5.0 {
            let (np, nl) = doc.add_page(Mm(W), Mm(H), "L");
            layer = doc.get_page(np).get_layer(nl);
            y = H - M;
            write_headers(&layer, &font_bold, &table.headers, col_w, y);
            y -= 5.0;
        }
        for (c, cell) in row.iter().enumerate() {
            let txt = truncate(cell, 30);
            layer.use_text(&txt, 8.0, Mm(M + c as f32 * col_w), Mm(y), &font);
        }
        y -= 4.5;
    }

    // 하단 페이지 번호는 printpdf 한계로 생략 — 페이지 수가 적은 감사 표본에선 불필요.
    let _ = header_y;
    doc.save_to_bytes().map_err(|e| format!("PDF 저장 실패: {e}"))
}

fn write_headers(
    layer: &PdfLayerReference,
    font: &IndirectFontRef,
    headers: &[String],
    col_w: f32,
    y: f32,
) {
    for (c, h) in headers.iter().enumerate() {
        let txt = truncate(h, 20);
        layer.use_text(&txt, 9.0, Mm(M + c as f32 * col_w), Mm(y), font);
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut t: String = s.chars().take(max - 1).collect();
        t.push('…');
        t
    }
}

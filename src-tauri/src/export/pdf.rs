//! PDF 내보내기 — A4 가로, 열 폭 비례 배분, 시스템 한글 폰트 자동 탐색.

use axon_core::domain::table::Table;
use printpdf::*;
use std::io::Cursor;

const W: f32 = 297.0;
const H: f32 = 210.0;
const MARGIN: f32 = 12.0;
const USABLE: f32 = W - 2.0 * MARGIN;
const BODY_PT: f32 = 6.5;
const HDR_PT: f32 = 7.0;
const META_TITLE_PT: f32 = 12.0;
const META_PT: f32 = 8.0;
const ROW_H: f32 = 3.8;
const HDR_GAP: f32 = 4.5;
const META_LINE_H: f32 = 4.0;

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

fn text_score(s: &str) -> f32 {
    s.chars()
        .map(|c| if c > '\u{2E7F}' { 1.7 } else { 1.0 })
        .sum()
}

fn col_widths(table: &Table) -> Vec<f32> {
    let n = table.headers.len().max(1);
    let mut scores: Vec<f32> = vec![0.0; n];
    for (i, h) in table.headers.iter().enumerate() {
        scores[i] = text_score(h);
    }
    for row in &table.rows {
        for (i, cell) in row.iter().enumerate().take(n) {
            scores[i] = scores[i].max(text_score(cell));
        }
    }
    for s in &mut scores {
        *s = s.clamp(3.0, 50.0);
    }
    let total: f32 = scores.iter().sum();
    if total == 0.0 {
        return vec![USABLE / n as f32; n];
    }
    scores.iter().map(|s| s / total * USABLE).collect()
}

fn max_chars(col_w: f32, font_pt: f32) -> usize {
    (col_w / (font_pt * 0.28)).max(3.0) as usize
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut t: String = s.chars().take(max.saturating_sub(1)).collect();
        t.push('…');
        t
    }
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

    let widths = col_widths(table);
    let mut y = H - MARGIN;
    let mut layer = doc.get_page(page1).get_layer(layer1);

    if !meta.is_empty() {
        layer.use_text("axon", META_TITLE_PT, Mm(MARGIN), Mm(y), &font_bold);
        y -= 6.0;
        for (k, v) in meta {
            layer.use_text(&format!("{k}: {v}"), META_PT, Mm(MARGIN), Mm(y), &font);
            y -= META_LINE_H;
        }
        y -= 3.0;
    }

    write_headers(&layer, &font_bold, &table.headers, &widths, y);
    y -= HDR_GAP;

    for row in &table.rows {
        if y < MARGIN + 4.0 {
            let (np, nl) = doc.add_page(Mm(W), Mm(H), "L");
            layer = doc.get_page(np).get_layer(nl);
            y = H - MARGIN;
            write_headers(&layer, &font_bold, &table.headers, &widths, y);
            y -= HDR_GAP;
        }
        let mut x = MARGIN;
        for (c, cell) in row.iter().enumerate() {
            let cw = widths.get(c).copied().unwrap_or(10.0);
            let mc = max_chars(cw, BODY_PT);
            layer.use_text(&truncate(cell, mc), BODY_PT, Mm(x), Mm(y), &font);
            x += cw;
        }
        y -= ROW_H;
    }

    doc.save_to_bytes().map_err(|e| format!("PDF 저장 실패: {e}"))
}

fn write_headers(
    layer: &PdfLayerReference,
    font: &IndirectFontRef,
    headers: &[String],
    widths: &[f32],
    y: f32,
) {
    let mut x = MARGIN;
    for (c, h) in headers.iter().enumerate() {
        let cw = widths.get(c).copied().unwrap_or(10.0);
        let mc = max_chars(cw, HDR_PT);
        layer.use_text(&truncate(h, mc), HDR_PT, Mm(x), Mm(y), font);
        x += cw;
    }
}

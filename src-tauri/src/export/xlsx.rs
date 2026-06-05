//! XlsxExporter — Table → .xlsx. 맑은 고딕·숫자서식(#,##0)·테두리·헤더 하늘색.
//! 연도 컬럼은 숫자로 기록해 엑셀 합계·수식이 바로 된다.

use axon_core::domain::table::Table;
use axon_core::port::exporter::Exporter;
use rust_xlsxwriter::{Color, Format, FormatAlign, FormatBorder, Workbook};

pub struct XlsxExporter;

impl Exporter for XlsxExporter {
    fn format(&self) -> &str {
        "xlsx"
    }

    fn export(&self, table: &Table) -> Vec<u8> {
        let mut wb = Workbook::new();
        let sheet = wb.add_worksheet();

        let font = "맑은 고딕";
        let header = Format::new()
            .set_font_name(font)
            .set_bold()
            .set_background_color(Color::RGB(0x00CCE5FF)) // 하늘색
            .set_border(FormatBorder::Thin)
            .set_align(FormatAlign::Center);
        let text = Format::new()
            .set_font_name(font)
            .set_border(FormatBorder::Thin);
        let number = Format::new()
            .set_font_name(font)
            .set_border(FormatBorder::Thin)
            .set_num_format("#,##0"); // 천단위 콤마, 소수 0

        for (c, h) in table.headers.iter().enumerate() {
            let _ = sheet.write_string_with_format(0, c as u16, h.as_str(), &header);
        }
        for (r, row) in table.rows.iter().enumerate() {
            let rr = (r + 1) as u32;
            for (c, cell) in row.iter().enumerate() {
                let cc = c as u16;
                // 연도 컬럼(2+)은 숫자화 → number 서식, 아니면 텍스트 서식
                if c >= 2 {
                    if let Ok(n) = cell.replace(',', "").parse::<f64>() {
                        let _ = sheet.write_number_with_format(rr, cc, n, &number);
                        continue;
                    }
                }
                let _ = sheet.write_string_with_format(rr, cc, cell.as_str(), &text);
            }
        }
        let _ = sheet.set_column_width(0, 14); // 재무제표 구분
        let _ = sheet.set_column_width(1, 28); // 계정명
        wb.save_to_buffer().unwrap_or_default()
    }
}

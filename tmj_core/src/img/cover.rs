// audience: internal
// # cover
// 覆盖与颜色混合：透明感知的 Cell 覆盖，以及 RGB 线性插值。
// 仅依赖 ratatui 的颜色与缓冲类型，不涉及图像解码。

use ratatui::buffer::{Buffer, Cell};
use ratatui::layout::Rect;
use ratatui::style::Color;

//// 混合两个 RGB 颜色 [@user 2026-06-06]
pub fn blend(mask_color: Color, cell_color: Color, percentage: f64) -> Color {
    let Color::Rgb(mask_red, mask_green, mask_blue) = mask_color else {
        return mask_color;
    };
    let Color::Rgb(cell_red, cell_green, cell_blue) = cell_color else {
        return mask_color;
    };

    let remain = 1.0 - percentage;

    let red = f64::from(mask_red).mul_add(percentage, f64::from(cell_red) * remain);
    let green = f64::from(mask_green).mul_add(percentage, f64::from(cell_green) * remain);
    let blue = f64::from(mask_blue).mul_add(percentage, f64::from(cell_blue) * remain);

    #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    Color::Rgb(red as u8, green as u8, blue as u8)
}

//// 用 mask_cell 覆盖 raw_cell [@user 2026-06-06]
// 仅当 mask_cell 有非空符号时执行覆盖；若 mask 的 fg/bg 被显式设置则一并复制。
pub fn cover_cell(raw_cell: &mut Cell, mask_cell: &Cell) {
    if mask_cell.symbol() == " "{
        return;
    }
    raw_cell.set_symbol(mask_cell.symbol());
    if mask_cell.fg != Color::Reset{
        raw_cell.set_fg(mask_cell.fg);
    }
    if mask_cell.bg != Color::Reset{
        raw_cell.set_bg(mask_cell.bg);
    }
}

//// 在 area 范围内用 new_buf 覆盖 raw_buf [@user 2026-06-06]
pub fn cover(raw_buf: &mut Buffer, new_buf: &mut Buffer, area: Rect) {
    for row in area.rows() {
        for col in row.columns() {
            let cell = &mut raw_buf[(col.x, col.y)];
            let mask_cell = &mut new_buf[(col.x, col.y)];
            cover_cell(cell, mask_cell);
        }
    }
}

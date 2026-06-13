// audience: internal
// # halfblock
// HalfBlock 格点的两层 Alpha 混合。每个终端格点被拆为上下两个半像素，
// 逐层执行 src * alpha + dst * (1 - alpha) 后重新编码为 ▀ ▄ 或空格。

use ratatui::buffer::{Buffer, Cell};
use ratatui::layout::Rect;
use ratatui::style::Color;

//// 混合两个 HalfBlock 格点，返回新的「字符 + 前景色 + 背景色」 [@user 2026-06-06]
// 混合公式：result = src * alpha + dst * (1 - alpha)
// - 透明像素（Color::Reset）会被视为透明层，允许底层颜色穿透。
// - 当 src 透明而上层无颜色时，直接保留 dst 对应半像素的颜色。
// - 当 src 有颜色而 dst 透明时，将 dst 视为黑色进行混合。
pub fn mix_half_block_cells(src: &Cell, dst: &Cell, alpha: f32) -> (char, Color, Color) {
    // src 为空/空格 → 完全透明，保留 dst 原样
    // if src.symbol().is_empty() || src.symbol() == " " {
    //     let dst_ch = dst.symbol().chars().next().unwrap_or(' ');
    //     return (dst_ch, dst.fg, dst.bg);
    // }
    // src 有真实文字字符且 dst 为空，保留 src
    // let src_ch = src.symbol().chars().next().unwrap_or(' ');
    // if src_ch != ' ' && src_ch != '▀' && src_ch != '▄' {
    //     if dst.symbol().is_empty() || dst.symbol() == " " {
    //         return (src_ch, src.fg, src.bg);
    //     }
    // }
    let (src_upper, src_lower) = decode_half_block_parts(src);
    let (dst_upper, dst_lower) = decode_half_block_parts(dst);

    let upper = mix_layer(src_upper, dst_upper, alpha);
    let lower = mix_layer(src_lower, dst_lower, alpha);

    encode_half_block(upper, lower)
}

// 覆盖一个cell内的两个hb格点
pub fn cover_half_block_cells(src: &Cell, dst: &Cell) -> (char, Color, Color) {
    // src 为空/空格 → 完全透明，保留 dst 原样
    if src.symbol().is_empty() || src.symbol() == " " {
        let dst_ch = dst.symbol().chars().next().unwrap_or(' ');
        return (dst_ch, dst.fg, dst.bg);
    }
    // src 有真实文字字符且 dst 为空，保留 src
    let src_ch = src.symbol().chars().next().unwrap_or(' ');
    if src_ch != ' ' && src_ch != '▀' && src_ch != '▄' {
        if dst.symbol().is_empty() || dst.symbol() == " " {
            return (src_ch, src.fg, src.bg);
        }
    }
    let (src_upper, src_lower) = decode_half_block_parts(src);
    let (dst_upper, dst_lower) = decode_half_block_parts(dst);

    let upper = cover_layer(src_upper, dst_upper);
    let lower = cover_layer(src_lower, dst_lower);

    encode_half_block(upper, lower)
}

//// 从 Cell 中提取上半像素和下半像素的颜色 [@user 2026-06-06]
// 返回 (upper, lower)：
// - Some(Color) 表示有颜色
// - None 表示透明（对应 Cell 中的 Color::Reset）
fn decode_half_block_parts(cell: &Cell) -> (Option<Color>, Option<Color>) {
    let fg = cell.fg;
    let bg = cell.bg;
    let ch = cell.symbol();

    // 当字符为空时视为空格，即完全透明
    if ch.is_empty() {
        return (None, None);
    }

    let ch = ch.chars().next().unwrap();

    match ch {
        ' ' => (None, None),
        '▄' => (None, color_if_not_reset(fg)),
        '▀' => (color_if_not_reset(fg), color_if_not_reset(bg)),
        // 其他字符（如 '█'）按全不透明处理：上下都用前景色
        _ => {
            let col = color_if_not_reset(fg);
            (col, col)
        }
    }
}

//// 辅助：如果颜色不是 Reset，返回 Some(color)，否则 None [@user 2026-06-06]
fn color_if_not_reset(c: Color) -> Option<Color> {
    match c {
        Color::Reset => None,
        other => Some(other),
    }
}

//// 将 Option<Color> 按 alpha 混合 [@user 2026-06-06]
// - src（上层）透明 → 直接返回 dst
// - src 不透明，dst 透明 → 将 dst 视作黑色后与 src 混合
// - 两者都不透明 → 标准混合 src * alpha + dst * (1 - alpha)
fn mix_layer(src: Option<Color>, dst: Option<Color>, alpha: f32) -> Option<Color> {
    match (src, dst) {
        (None, _) => dst,
        (Some(sc), None) => Some(mix_color(sc, Color::Rgb(0, 0, 0), alpha)),
        (Some(sc), Some(dc)) => Some(mix_color(sc, dc, alpha)),
    }
}

fn cover_layer(src: Option<Color>, dst: Option<Color>) -> Option<Color> {
    match (src, dst) {
        (None, _) => dst,
        _ => src,
    }
}

//// 混合两个不透明 RGB 颜色 [@user 2026-06-06]
fn mix_color(c1: Color, c2: Color, alpha: f32) -> Color {
    let (r1, g1, b1) = rgb_from_color(c1);
    let (r2, g2, b2) = rgb_from_color(c2);
    let inv = 1.0 - alpha;
    Color::Rgb(
        (r1 as f32 * alpha + r2 as f32 * inv).round() as u8,
        (g1 as f32 * alpha + g2 as f32 * inv).round() as u8,
        (b1 as f32 * alpha + b2 as f32 * inv).round() as u8,
    )
}

//// 从 Color 中提取 RGB 分量 [@user 2026-06-06]
// Reset 被当作黑色处理，但此函数不应接收 Reset。
fn rgb_from_color(c: Color) -> (u8, u8, u8) {
    match c {
        Color::Rgb(r, g, b) => (r, g, b),
        _ => (0, 0, 0),
    }
}

//// 将两个 Optional 颜色编码回 HalfBlock 字符 [@user 2026-06-06]
fn encode_half_block(upper: Option<Color>, lower: Option<Color>) -> (char, Color, Color) {
    match (upper, lower) {
        (None, None) => (' ', Color::Reset, Color::Reset),
        (None, Some(lc)) => ('▄', lc, Color::Reset),
        (Some(uc), None) => ('▀', uc, Color::Reset),
        (Some(uc), Some(lc)) => ('▀', uc, lc),
    }
}

//// 将混合结果直接写入一个 Cell [@user 2026-06-06]
pub fn mix_into_cell(src: &Cell, dst: &Cell, alpha: f32, out: &mut Cell) {
    let (ch, fg, bg) = mix_half_block_cells(src, dst, alpha);
    if ch == ' ' {
        return;
    }
    let mut buf = [0; 4];
    let s = ch.encode_utf8(&mut buf);
    out.set_symbol(s);
    if fg != Color::Reset {
        out.set_fg(fg);
    }
    if bg != Color::Reset {
        out.set_bg(bg);
    }
}

pub fn cover_into_cell(src: &Cell, dst: &Cell, out: &mut Cell) {
    let (ch, fg, bg) = cover_half_block_cells(src, dst);
    if ch == ' ' {
        return;
    }
    let mut buf = [0; 4];
    let s = ch.encode_utf8(&mut buf);
    out.set_symbol(s);
    if fg != Color::Reset {
        out.set_fg(fg);
    }
    if bg != Color::Reset {
        out.set_bg(bg);
    }
}

//// 用 HalfBlock 混合做全量覆盖，兼容 `CoverMethod` 签名 [@user 2026-06-06]
// 对 area 内每对格点执行 `mix_into_cell(new, old, 1.0, out)`，
// 将 new_buf 的图像以半像素精度覆盖到 raw_buf 上。
pub fn cover_halfblock(raw_buf: &mut Buffer, new_buf: &mut Buffer, area: Rect) {
    for row in area.rows() {
        for col in row.columns() {
            let old = raw_buf[(col.x, col.y)].clone();
            let mask = new_buf[(col.x, col.y)].clone();
            cover_into_cell(&mask, &old, &mut raw_buf[(col.x, col.y)]);
        }
    }
}

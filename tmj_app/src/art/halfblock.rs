
use ratatui::buffer::Cell;
use ratatui::style::Color;

/// 混合两个 HalfBlock 格点，返回新的“字符 + 前景色 + 背景色”。
///
/// 混合公式：`result = src * alpha + dst * (1 - alpha)`
/// - 透明像素（Color::Reset）会被视为透明层，允许底层颜色穿透。
/// - 当 src 透明而上层无颜色时，直接保留 dst 对应半像素的颜色。
/// - 当 src 有颜色而 dst 透明时，将 dst 视为黑色（Rgb(0, 0, 0)）进行混合。
///
/// 注意：这里的“透明”指 Color::Reset，与终端默认背景不同，后者可能需要额外处理。
pub fn mix_half_block_cells(src: &Cell, dst: &Cell, alpha: f32) -> (char, Color, Color) {
    // 解码源格点
    let (src_upper, src_lower) = decode_half_block_parts(src);
    // 解码目标格点
    let (dst_upper, dst_lower) = decode_half_block_parts(dst);

    // 逐层混合
    let upper = mix_layer(src_upper, dst_upper, alpha);
    let lower = mix_layer(src_lower, dst_lower, alpha);

    // 重新编码为 HalfBlock 格点
    encode_half_block(upper, lower)
}

/// 从 Cell 中提取上半像素和下半像素的颜色。
/// 返回 (upper, lower)：
/// - Some(Color) 表示有颜色
/// - None 表示透明（对应 Cell 中的 Color::Reset）
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
        '▄' => (None, color_if_not_reset(fg)), // 下半块：颜色在前景色
        //
        '▀' => (
            color_if_not_reset(fg), // 上半块：前景色
            color_if_not_reset(bg), // 下半块：背景色
        ),
        // 其他字符（如 '█'）理论上也可能出现，按全不透明处理：上下都用前景色
        _ => {
            let col = color_if_not_reset(fg);
            (col, col)
        }
    }
}

/// 辅助：如果颜色不是 Reset，返回 Some(color)，否则 None
fn color_if_not_reset(c: Color) -> Option<Color> {
    match c {
        Color::Reset => None,
        other => Some(other),
    }
}

/// 将 Option<Color> 按 alpha 混合。
/// - src（上层）透明 → 直接返回 dst
/// - src 不透明，dst 透明 → 将 dst 视作黑色 (0,0,0) 后与 src 混合
/// - 两者都不透明 → 标准混合 src * alpha + dst * (1 - alpha)
fn mix_layer(src: Option<Color>, dst: Option<Color>, alpha: f32) -> Option<Color> {
    match (src, dst) {
        (None, _) => dst,
        (Some(sc), None) => Some(mix_color(sc, Color::Rgb(0, 0, 0), alpha)),
        (Some(sc), Some(dc)) => Some(mix_color(sc, dc, alpha)),
    }
}

/// 混合两个不透明 RGB 颜色
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

/// 从 Color 中提取 RGB 分量，Reset 当黑色（但此函数不应接收 Reset）
fn rgb_from_color(c: Color) -> (u8, u8, u8) {
    match c {
        Color::Rgb(r, g, b) => (r, g, b),
        _ => (0, 0, 0),
    }
}

/// 将两个 Optional 颜色编码回 HalfBlock 字符
fn encode_half_block(upper: Option<Color>, lower: Option<Color>) -> (char, Color, Color) {
    match (upper, lower) {
        (None, None) => (' ', Color::Reset, Color::Reset),
        (None, Some(lc)) => ('▄', lc, Color::Reset),
        (Some(uc), None) => ('▀', uc, Color::Reset),
        (Some(uc), Some(lc)) => ('▀', uc, lc),
    }
}

/// 将混合结果直接写入一个 Cell，方便使用
pub fn mix_into_cell(src: &Cell, dst: &Cell, alpha: f32, out: &mut Cell) {
    let (ch, fg, bg) = mix_half_block_cells(src, dst, alpha);
    out.set_symbol(&ch.to_string());
    out.fg = fg;
    out.bg = bg;
}

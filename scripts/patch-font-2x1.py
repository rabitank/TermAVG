"""把等宽 CJK 字体打成精确 2:1 单元格，供 half-block 像素画使用。

处理三件事：
  1. 垂直度量改为 ascender - descender = 2 * ASCII advance，使 ratatui-wgpu 的
     char_width = floor(advance * h / (asc - desc)) 恰为 h / 2。
  2. 块元素字形 ▀ ▄ █ 重画为精确的半格/整格矩形，与新行框对齐，避免拼接缝隙。
  3. 补齐 MapleMono 缺失的圆形符号 ⚪(U+26AA) ⭕(U+2B55)，画成空心圆环。

用法: python patch-font-2x1.py <input.ttf> <output.ttf>
"""

import math
import sys

from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.ttLib import TTFont


def rect(x0: int, y0: int, x1: int, y1: int):
    pen = TTGlyphPen(None)
    pen.moveTo((x0, y0))
    pen.lineTo((x1, y0))
    pen.lineTo((x1, y1))
    pen.lineTo((x0, y1))
    pen.closePath()
    return pen.glyph()


def _circle(pen: TTGlyphPen, cx: int, cy: int, r: int, ccw: bool, steps: int = 96) -> None:
    sign = 1.0 if ccw else -1.0
    pts = [
        (round(cx + r * math.cos(sign * 2 * math.pi * i / steps)),
         round(cy + r * math.sin(sign * 2 * math.pi * i / steps)))
        for i in range(steps)
    ]
    pen.moveTo(pts[0])
    for p in pts[1:]:
        pen.lineTo(p)
    pen.closePath()


def ring(cx: int, cy: int, r_out: int, r_in: int):
    pen = TTGlyphPen(None)
    _circle(pen, cx, cy, r_out, ccw=True)
    _circle(pen, cx, cy, r_in, ccw=False)
    return pen.glyph()


def add_glyph(font: TTFont, name: str, glyph, adv: int) -> None:
    glyf = font["glyf"]
    order = list(font.getGlyphOrder())
    if name not in order:
        order.append(name)
        font.setGlyphOrder(order)
    glyph.recalcBounds(glyf)
    glyf[name] = glyph
    font["hmtx"][name] = (adv, 0)
    font["maxp"].numGlyphs = len(order)


def set_cmap(font: TTFont, cp: int, name: str) -> None:
    for t in font["cmap"].tables:
        if t.isUnicode():
            t.cmap[cp] = name


def patch(src: str, dst: str) -> None:
    font = TTFont(src)
    cmap = font.getBestCmap()
    glyf = font["glyf"]
    hmtx = font["hmtx"]

    adv = hmtx[cmap[ord("m")]][0]
    asc = adv * 3 // 2
    desc = -(adv // 2)
    mid = (asc + desc) // 2

    os2 = font["OS/2"]
    os2.sTypoAscender = asc
    os2.sTypoDescender = desc
    os2.sTypoLineGap = 0
    os2.usWinAscent = asc
    os2.usWinDescent = -desc
    hhea = font["hhea"]
    hhea.ascent = asc
    hhea.descent = desc
    hhea.lineGap = 0

    for cp, (y0, y1) in {
        0x2580: (mid, asc),   # ▀ 上半格
        0x2584: (desc, mid),  # ▄ 下半格
        0x2588: (desc, asc),  # █ 整格
    }.items():
        name = cmap.get(cp)
        if name is None:
            print(f"  skip U+{cp:04X}: missing")
            continue
        glyph = rect(0, y0, adv, y1)
        glyph.recalcBounds(glyf)
        glyf[name] = glyph
        hmtx[name] = (adv, 0)
        print(f"  patched U+{cp:04X} {name}: x[0,{adv}] y[{y0},{y1}]")

    # 缺失的圆形符号，画成空心圆环；圆心对齐其它几何符号
    cx, cy = adv // 2, adv * 3 // 5
    for cp, name, r_out, r_in in [
        (0x26AA, "uni26AA", 250 * adv // 600, 185 * adv // 600),  # ⚪ 中号白圆
        (0x2B55, "uni2B55", 295 * adv // 600, 175 * adv // 600),  # ⭕ 大号粗圆
    ]:
        if cp in cmap:
            print(f"  skip U+{cp:04X}: already present")
            continue
        add_glyph(font, name, ring(cx, cy, r_out, r_in), adv)
        set_cmap(font, cp, name)
        print(f"  added U+{cp:04X} {name}: ring r[{r_in},{r_out}] @({cx},{cy})")

    print(f"  metrics: asc={asc} desc={desc} H={asc - desc} adv={adv} ratio={adv / (asc - desc):.4f}")
    font.save(dst)


if __name__ == "__main__":
    patch(sys.argv[1], sys.argv[2])

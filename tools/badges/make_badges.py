"""Draws the download buttons of the README that stand beside the App Store badge: one per platform and language.

Run from the repository root:  uv run --no-project --with fonttools --with uharfbuzz python tools/badges/make_badges.py
The logos in tools/badges/logos are from Simple Icons 16.38.1 (CC0); the one of macOS is a laptop around the apple of
the App Store badge. The text is turned into outlines, so the buttons look the same in every browser: Inter as in the
app, and Noto Sans for Japanese, Chinese, Korean and Hindi.
"""

import re
import sys
import urllib.request
from pathlib import Path

import uharfbuzz as hb
from fontTools.pens.boundsPen import BoundsPen
from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.pens.transformPen import TransformPen
from fontTools.svgLib.path import parse_path
from fontTools.ttLib import TTFont

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools" / "i18n"))
from make_fonts import CACHE, FONTS, SOURCE  # noqa: E402

OUT = ROOT / "assets" / "badges"
LOGOS = Path(__file__).resolve().parent / "logos"
INTER = ROOT / "crates" / "fractique-app" / "assets" / "fonts"

PLATFORMS = {"windows": "Windows", "macos": "macOS", "linux": "Linux", "android": "Android"}
LOGO = {"windows": "windows", "linux": "linux", "android": "android"}
COLORS = {
    "windows": ("#0078d4", "#fff"),
    "macos": ("#e5e5ea", "#1d1d1f"),
    "linux": ("#fcc624", "#1d1d1f"),
    "android": ("#3ddc84", "#073042"),
}

LANGUAGES = {
    "en-us": ("Download for", None),
    "uk-ua": ("Завантажити для", None),
    "ru-ru": ("Скачать для", None),
    "de-de": ("Download für", None),
    "fr-fr": ("Télécharger pour", None),
    "es-es": ("Descargar para", None),
    "it-it": ("Scarica per", None),
    "ja-jp": ("ダウンロード", "ja"),
    "zh-cn": ("下载", "zh-Hans"),
    "pt-br": ("Baixar para", None),
    "ko-kr": ("다운로드", "ko"),
    "hi-in": ("डाउनलोड करें", "hi"),
    "tr-tr": ("İndir", None),
    "pl-pl": ("Pobierz na", None),
    "id-id": ("Unduh untuk", None),
}

HEIGHT = 40
APP_STORE = OUT / "app-store-en-us.svg"
# Measured on Apple's badge, so the buttons line up with it: where the logo and the two lines sit, their sizes in Inter.
LOGO_CENTER = (18.83, 19.61)
LOGO_FIT = {"macos": (21, 21.8), "windows": (17.5, 17.5), "linux": (21.8, 21.8), "android": (20, 21.8)}
TEXT_X, RIGHT = 34.43, 10.17
TOP = (8.23, 14.7, 0.276)
NAME = (17.08, 30.5, -0.605)


def number(value):
    text = f"{value:.2f}".rstrip("0").rstrip(".")
    return "0" if text == "-0" else text


def font_file(code):
    if code is None:
        return None
    path, name, _ = FONTS[code]
    full = CACHE / name.replace("-subset", "")
    if not full.exists():
        CACHE.mkdir(parents=True, exist_ok=True)
        urllib.request.urlretrieve(f"{SOURCE}/{path}", full)
    return full


def outline(path, text, size, weight, baseline, tracking):
    font = TTFont(path)
    location = None
    if "fvar" in font:
        location = {axis.axisTag: axis.defaultValue for axis in font["fvar"].axes}
        location["wght"] = weight
    glyphs = font.getGlyphSet(location=location)
    order = font.getGlyphOrder()
    scale = size / font["head"].unitsPerEm

    shaper = hb.Font(hb.Face(hb.Blob.from_file_path(str(path))))
    if location:
        shaper.set_variations(location)
    buffer = hb.Buffer()
    buffer.add_str(text)
    buffer.guess_segment_properties()
    hb.shape(shaper, buffer)

    placed = []
    advance = 0
    for index, (info, position) in enumerate(zip(buffer.glyph_infos, buffer.glyph_positions)):
        left = (advance + position.x_offset) * scale + index * tracking
        placed.append((order[info.codepoint], left, baseline - position.y_offset * scale))
        advance += position.x_advance

    bounds = BoundsPen(glyphs)
    for name, left, top in placed:
        glyphs[name].draw(TransformPen(bounds, (scale, 0, 0, -scale, left, top)))
    shift = TEXT_X - bounds.bounds[0]

    pen = SVGPathPen(glyphs, ntos=number)
    for name, left, top in placed:
        glyphs[name].draw(TransformPen(pen, (scale, 0, 0, -scale, left + shift, top)))
    return pen.getCommands(), bounds.bounds[2] + shift


def rounded(x0, y0, x1, y1, top, bottom):
    return (
        f"M{x0 + top} {y0}H{x1 - top}A{top} {top} 0 0 1 {x1} {y0 + top}V{y1 - bottom}A{bottom} {bottom} 0 0 1 {x1 - bottom} {y1}"
        f"H{x0 + bottom}A{bottom} {bottom} 0 0 1 {x0} {y1 - bottom}V{y0 + top}A{top} {top} 0 0 1 {x0 + top} {y0}Z"
    )


def laptop():
    apple = "".join(re.findall(r'id="_Path_2?" [^>]*?d="([^"]+)"', APP_STORE.read_text(encoding="utf-8")))
    bounds = BoundsPen(None)
    parse_path(apple, bounds)
    x0, y0, x1, y1 = bounds.bounds
    scale = 7.2 / (y1 - y0)
    pen = SVGPathPen(None, ntos=number)
    parse_path(apple, TransformPen(pen, (scale, 0, 0, scale, 12 - (x0 + x1) / 2 * scale, 10.5 - (y0 + y1) / 2 * scale)))
    screen = rounded(3, 3.5, 21, 17, 1.6, 0.4) + rounded(4.6, 5.1, 19.4, 15.6, 0.4, 0.4)
    return screen + rounded(0.5, 17.6, 23.5, 19.6, 0.3, 1) + pen.getCommands()


def logo(platform):
    path = laptop() if platform == "macos" else re.search(r' d="([^"]+)"', (LOGOS / f"{LOGO[platform]}.svg").read_text(encoding="utf-8")).group(1)
    bounds = BoundsPen(None)
    parse_path(path, bounds)
    x0, y0, x1, y1 = bounds.bounds
    width, height = LOGO_FIT[platform]
    scale = min(width / (x1 - x0), height / (y1 - y0))
    dx = LOGO_CENTER[0] - (x0 + x1) / 2 * scale
    dy = LOGO_CENTER[1] - (y0 + y1) / 2 * scale
    pen = SVGPathPen(None, ntos=number)
    parse_path(path, TransformPen(pen, (scale, 0, 0, scale, dx, dy)))
    return pen.getCommands()


def badge(platform, top, top_font, tracked):
    top_path, top_right = outline(top_font, top, TOP[0], 500, TOP[1], TOP[2] if tracked else 0)
    name_path, name_right = outline(INTER / "Inter-Medium.ttf", PLATFORMS[platform], NAME[0], 500, NAME[1], NAME[2])
    width = number(max(top_right, name_right) + RIGHT)
    background, ink = COLORS[platform]
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{HEIGHT}" viewBox="0 0 {width} {HEIGHT}">\n'
        f"  <title>{top} {PLATFORMS[platform]}</title>\n"
        f'  <rect width="{width}" height="{HEIGHT}" rx="6.5" fill="{background}"/>\n'
        f'  <path fill="{ink}" fill-rule="evenodd" d="{logo(platform)}"/>\n'
        f'  <path fill="{ink}" d="{top_path}"/>\n'
        f'  <path fill="{ink}" d="{name_path}"/>\n'
        "</svg>\n"
    )


def main():
    for language, (top, code) in LANGUAGES.items():
        top_font = font_file(code) or INTER / "Inter-Medium.ttf"
        for platform in PLATFORMS:
            (OUT / f"{platform}-{language}.svg").write_text(badge(platform, top, top_font, code is None), encoding="utf-8")
    print(f"{len(LANGUAGES) * len(PLATFORMS)} buttons in {OUT.relative_to(ROOT)}")


if __name__ == "__main__":
    main()

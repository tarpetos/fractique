"""Draws the app icon and writes every size and format the platforms need.

Run from the repository root:  uv run --no-project --with pillow python tools/icon/make_icons.py
"""

from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "assets" / "icon"
ANDROID = ROOT / "crates" / "fractique-app" / "android" / "res"

SIZE = 1024
OVERSAMPLE = 4
DARK = "#1a1433"
LIGHT = "#f1effc"
WARM = ["#ffcc55", "#ff8a50", "#f04e8c", "#a855f7", "#5b6cff"]
DEEP = ["#f5a623", "#f2682f", "#e0357a", "#8f3ff0", "#4658f0"]
WHITE = ["#ffffff"] * 5
LEVELS = [(128, 24, 30), (64, 12, 15), (32, 6, 7.5)]
DOT = 8


def glyph(colors, dot):
    shapes, x = [], 0
    for cell, gap, radius in LEVELS:
        step = cell + gap
        for row in range(5):
            shapes.append((x, row * step, cell, radius, colors[row]))
        shapes.append((x + step, 0, cell, radius, colors[0]))
        shapes.append((x + step, 2 * step, cell, radius, colors[2]))
        x += 2 * step
    width = x + 2 * DOT
    height = 5 * LEVELS[0][0] + 4 * LEVELS[0][1]
    return shapes, (x, DOT, dot), width, height


def draw(background, corner, colors, dot, scale=1.0):
    side = SIZE * OVERSAMPLE
    image = Image.new("RGBA", (side, side), (0, 0, 0, 0))
    pen = ImageDraw.Draw(image)
    if background:
        pen.rounded_rectangle((0, 0, side - 1, side - 1), corner * OVERSAMPLE, fill=background)
    shapes, (dot_x, dot_r, dot_color), width, height = glyph(colors, dot)
    k = OVERSAMPLE * scale
    left, top = (side - width * k) / 2, (side - height * k) / 2
    for x, y, cell, radius, color in shapes:
        box = (left + x * k, top + y * k, left + (x + cell) * k, top + (y + cell) * k)
        pen.rounded_rectangle(box, radius * k, fill=color)
    cx, cy = left + (dot_x + dot_r) * k, top + dot_r * k
    pen.ellipse((cx - dot_r * k, cy - dot_r * k, cx + dot_r * k, cy + dot_r * k), fill=dot_color)
    return image.resize((SIZE, SIZE), Image.LANCZOS)


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    icon = draw(DARK, 230, WARM, "#ffffff")
    icon.save(OUT / "icon-1024.png")
    for side in (16, 32, 48, 64, 128, 256, 512):
        icon.resize((side, side), Image.LANCZOS).save(OUT / f"icon-{side}.png")
    icon.save(OUT / "icon.ico", sizes=[(s, s) for s in (16, 24, 32, 48, 64, 128, 256)])
    icon.save(OUT / "icon.icns")
    draw(LIGHT, 230, DEEP, DARK).save(OUT / "icon-light-1024.png")
    draw(DARK, 230, WHITE, "#ffffff").save(OUT / "icon-mono-1024.png")
    draw(DARK, 0, WARM, "#ffffff").convert("RGB").save(OUT / "ios-1024.png")

    # Android crops an adaptive icon's foreground to the middle two thirds, so the glyph is drawn smaller there.
    foreground = draw(None, 0, WARM, "#ffffff", scale=0.62)
    for density, legacy, adaptive in (("mdpi", 48, 108), ("hdpi", 72, 162), ("xhdpi", 96, 216), ("xxhdpi", 144, 324), ("xxxhdpi", 192, 432)):
        folder = ANDROID / f"mipmap-{density}"
        folder.mkdir(parents=True, exist_ok=True)
        icon.resize((legacy, legacy), Image.LANCZOS).save(folder / "ic_launcher.png")
        foreground.resize((adaptive, adaptive), Image.LANCZOS).save(folder / "ic_launcher_foreground.png")


if __name__ == "__main__":
    main()

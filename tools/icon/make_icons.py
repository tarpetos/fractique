"""Draws the app icon and writes every size and format the platforms need.

Run from the repository root:  uv run --no-project --with pillow python tools/icon/make_icons.py
"""

from pathlib import Path

from PIL import Image, ImageChops, ImageDraw, ImageFilter

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "assets" / "icon"
ANDROID = ROOT / "crates" / "fractique-app" / "android" / "res"

SIZE = 1024
OVERSAMPLE = 2
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


def rounded(size, radius, inset=0):
    mask = Image.new("L", size, 0)
    box = (inset, inset, size[0] - 1 - inset, size[1] - 1 - inset)
    ImageDraw.Draw(mask).rounded_rectangle(box, max(radius - inset, 0), fill=255)
    return mask


def ramp(size, stops):
    """Opacity along the diagonal, from the lit upper left corner to the lower right one."""
    down = Image.linear_gradient("L").resize(size)
    across = down.rotate(90, expand=True).transpose(Image.FLIP_TOP_BOTTOM).resize(size)
    diagonal = ImageChops.add(down, across, scale=2)
    first, middle, last = stops
    return diagonal.point(lambda v: int(first + (middle - first) * v / 128 if v < 128 else middle + (last - middle) * (v - 128) / 127))


def layer(size, color, alpha):
    out = Image.new("RGBA", size, color)
    out.putalpha(alpha)
    return out


def mix(color, other, share):
    one = Image.blend(Image.new("RGB", (1, 1), color), Image.new("RGB", (1, 1), other), share)
    return one.getpixel((0, 0))


def shaded(size, top, bottom):
    return Image.composite(Image.new("RGB", size, bottom), Image.new("RGB", size, top), Image.linear_gradient("L").resize(size))


def specular(size, radius, width, stops, soft):
    """Light caught by the edge of a glass shape: bright at the upper left, weaker at the lower right, none between."""
    ring = ImageChops.subtract(rounded(size, radius), rounded(size, radius, width))
    if soft:
        ring = ring.filter(ImageFilter.GaussianBlur(soft))
    return ImageChops.multiply(ImageChops.multiply(ring, rounded(size, radius)), ramp(size, stops))


def glass_tile(image, box, radius, color):
    left, upper, right, lower = (int(round(v)) for v in box)
    side = right - left
    size = (side, side)
    mask = rounded(size, radius)

    reach = int(side * 0.3)
    shadow = Image.new("RGBA", (side + 2 * reach, side + 2 * reach), (0, 0, 0, 0))
    shadow.paste(layer(size, mix(color, "#000000", 0.35), mask.point(lambda v: int(v * 0.6))), (reach, reach + int(side * 0.055)))
    image.alpha_composite(shadow.filter(ImageFilter.GaussianBlur(side * 0.05)), (left - reach, upper - reach))

    tile = shaded(size, mix(color, "#ffffff", 0.26), mix(color, "#000000", 0.1)).convert("RGBA")
    tile.alpha_composite(layer(size, "#ffffff", specular(size, radius, int(side * 0.09), (95, 0, 55), side * 0.035)))
    tile.alpha_composite(layer(size, "#ffffff", specular(size, radius, max(int(side * 0.018), 2), (245, 0, 170), 0)))
    tile.putalpha(mask.point(lambda v: int(v * 0.95)))
    image.alpha_composite(tile, (left, upper))


def draw(background, corner, colors, dot, scale=1.0):
    side = SIZE * OVERSAMPLE
    size = (side, side)
    image = Image.new("RGBA", size, (0, 0, 0, 0))
    if background:
        deep = 0.3 if background == DARK else 0.07
        image = shaded(size, mix(background, "#ffffff", 0.12), mix(background, "#000000", deep)).convert("RGBA")
        rim = specular(size, corner * OVERSAMPLE, int(side * 0.006), (120, 0, 70), 0)
        image.alpha_composite(layer(size, "#ffffff", rim))
    shapes, (dot_x, dot_r, dot_color), width, height = glyph(colors, dot)
    k = OVERSAMPLE * scale
    left, top = (side - width * k) / 2, (side - height * k) / 2
    for x, y, cell, radius, color in shapes:
        glass_tile(image, (left + x * k, top + y * k, left + (x + cell) * k, top + (y + cell) * k), radius * k, color)
    cx, cy = left + (dot_x + dot_r) * k, top + dot_r * k
    ImageDraw.Draw(image).ellipse((cx - dot_r * k, cy - dot_r * k, cx + dot_r * k, cy + dot_r * k), fill=dot_color)
    if background:
        image.putalpha(rounded(size, corner * OVERSAMPLE))
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

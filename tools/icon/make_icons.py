"""Draws the app icon and writes every size and format the platforms need.

Run from the repository root:  uv run --no-project --with pillow python tools/icon/make_icons.py
"""

from pathlib import Path

from PIL import Image, ImageChops, ImageDraw, ImageEnhance, ImageFilter, ImageOps

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "assets" / "icon"
ANDROID = ROOT / "crates" / "fractique-app" / "android" / "res"
# Rendered from flame.json: fractique render tools/icon/flame.json --size 1024x1024 --quality high -o tools/icon/flame.png
FLAME = Path(__file__).with_name("flame.png")

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


def rounded(size, radius):
    mask = Image.new("L", size, 0)
    ImageDraw.Draw(mask).rounded_rectangle((0, 0, size[0] - 1, size[1] - 1), radius, fill=255)
    return mask


def ramp(size, stops):
    """Opacity along the diagonal, from the lit upper left corner to the lower right one."""
    down = Image.linear_gradient("L").resize(size)
    across = down.rotate(90, expand=True).transpose(Image.FLIP_TOP_BOTTOM).resize(size)
    diagonal = ImageChops.add(down, across, scale=2)
    first, middle, last = stops
    return diagonal.point(lambda v: int(first + (middle - first) * v / 128 if v < 128 else middle + (last - middle) * (v - 128) / 127))


def glass_tile(image, backdrop, box, radius, color):
    """A tile of clear tinted glass: what is behind it shows through magnified and blurred, the edge catches light."""
    left, upper, right, lower = (int(round(v)) for v in box)
    side = right - left
    size = (side, side)
    mask = rounded(size, radius)

    reach = int(side * 0.3)
    shadow = Image.new("RGBA", (side + 2 * reach, side + 2 * reach), (0, 0, 0, 0))
    shadow.paste((0, 0, 0, 150), (reach, reach + int(side * 0.06)), mask)
    image.alpha_composite(shadow.filter(ImageFilter.GaussianBlur(side * 0.07)), (left - reach, upper - reach))

    inset = side * 0.12
    seen = backdrop.crop((left, upper, right, lower)).resize(size, Image.BICUBIC, box=(inset, inset, side - inset, side - inset))
    seen = seen.filter(ImageFilter.GaussianBlur(side * 0.012))
    seen = ImageEnhance.Brightness(seen).enhance(1.7)
    lifted = Image.blend(seen, Image.new("RGB", size, "#ffffff"), 0.5)
    tile = ImageChops.multiply(lifted, Image.new("RGB", size, color)).convert("RGBA")

    # Glass is darker where it is thick at the lower edge and clearer in the middle.
    depth = Image.new("RGBA", size, (0, 0, 0, 0))
    depth.putalpha(ramp(size, (0, 0, 70)))
    tile.alpha_composite(depth)

    gloss = Image.new("RGBA", size, (255, 255, 255, 0))
    gloss.putalpha(ImageChops.multiply(mask, ramp(size, (90, 0, 0))))
    tile.alpha_composite(gloss)

    width = max(side * 0.045, 2)
    inner = Image.new("L", size, 0)
    inner.paste(rounded((int(side - 2 * width), int(side - 2 * width)), max(radius - width, 0)), (int(width), int(width)))
    ring = ImageChops.subtract(mask, inner).filter(ImageFilter.GaussianBlur(width * 0.35))
    rim = Image.new("RGBA", size, (255, 255, 255, 0))
    rim.putalpha(ImageChops.multiply(ring, ramp(size, (235, 30, 120))))
    tile.alpha_composite(rim)

    tile.putalpha(mask)
    image.alpha_composite(tile, (left, upper))


def plate(side, background, flame):
    base = Image.new("RGB", (side, side), background)
    if flame == "none":
        return base
    picture = Image.open(FLAME).convert("RGB").resize((side, side), Image.LANCZOS)
    if flame == "grey":
        picture = ImageOps.grayscale(picture).convert("RGB")
    return picture


def draw(background, corner, colors, dot, scale=1.0, flame="colour"):
    side = SIZE * OVERSAMPLE
    shapes, (dot_x, dot_r, dot_color), width, height = glyph(colors, dot)
    k = OVERSAMPLE * scale
    left, top = (side - width * k) / 2, (side - height * k) / 2

    backdrop = plate(side, background, flame)
    if flame == "none":
        glow = Image.new("RGBA", (side, side), (0, 0, 0, 0))
        pen = ImageDraw.Draw(glow)
        for x, y, cell, radius, color in shapes:
            pen.rounded_rectangle((left + x * k, top + y * k, left + (x + cell) * k, top + (y + cell) * k), radius * k, fill=color)
        glow = glow.filter(ImageFilter.GaussianBlur(side * 0.05))
        backdrop = Image.composite(glow.convert("RGB"), backdrop, glow.getchannel("A").point(lambda v: int(v * 0.6)))
    image = backdrop.convert("RGBA")

    for x, y, cell, radius, color in shapes:
        glass_tile(image, backdrop, (left + x * k, top + y * k, left + (x + cell) * k, top + (y + cell) * k), radius * k, color)
    cx, cy = left + (dot_x + dot_r) * k, top + dot_r * k
    ImageDraw.Draw(image).ellipse((cx - dot_r * k, cy - dot_r * k, cx + dot_r * k, cy + dot_r * k), fill=dot_color)
    image.putalpha(rounded((side, side), corner * OVERSAMPLE))
    return image.resize((SIZE, SIZE), Image.LANCZOS)


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    icon = draw(DARK, 230, WARM, "#ffffff")
    icon.save(OUT / "icon-1024.png")
    for side in (16, 32, 48, 64, 128, 256, 512):
        icon.resize((side, side), Image.LANCZOS).save(OUT / f"icon-{side}.png")
    icon.save(OUT / "icon.ico", sizes=[(s, s) for s in (16, 24, 32, 48, 64, 128, 256)])
    icon.save(OUT / "icon.icns")
    draw(LIGHT, 230, DEEP, DARK, flame="none").save(OUT / "icon-light-1024.png")
    draw(DARK, 230, WHITE, "#ffffff", flame="grey").save(OUT / "icon-mono-1024.png")
    draw(DARK, 0, WARM, "#ffffff").convert("RGB").save(OUT / "ios-1024.png")

    # Android crops an adaptive icon's foreground to the middle two thirds, so the glyph is drawn smaller there.
    foreground = draw(DARK, 0, WARM, "#ffffff", scale=0.62)
    for density, legacy, adaptive in (("mdpi", 48, 108), ("hdpi", 72, 162), ("xhdpi", 96, 216), ("xxhdpi", 144, 324), ("xxxhdpi", 192, 432)):
        folder = ANDROID / f"mipmap-{density}"
        folder.mkdir(parents=True, exist_ok=True)
        icon.resize((legacy, legacy), Image.LANCZOS).save(folder / "ic_launcher.png")
        foreground.resize((adaptive, adaptive), Image.LANCZOS).save(folder / "ic_launcher_foreground.png")


if __name__ == "__main__":
    main()

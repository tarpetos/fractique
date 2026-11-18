"""Draws the app icon and writes every size and format the platforms need.

Run from the repository root:  uv run --no-project --with pillow --with numpy python tools/icon/make_icons.py
"""

from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, ImageFilter, ImageOps

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "assets" / "icon"
ANDROID = ROOT / "crates" / "fractique-app" / "android" / "res"
# What the glass is seen against: fractique render tools/icon/flame.json --size 1024x1024 --quality high -o tools/icon/flame.png
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
CLEAR = 0.55


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


def glass(image, box, radius, bezel, refraction, blur, opacity, depth, tint, shadow, peak=0.0):
    """One pane of the app's glass (crates/fractique-app/src/glass.wgsl), drawn over the image."""
    side = image.shape[0]
    soft = np.asarray(Image.fromarray((image * 255).astype(np.uint8)).filter(ImageFilter.GaussianBlur(side * 0.02)), dtype=np.float32) / 255
    reach, strength = shadow
    x0, y0 = max(int(box[0] - 3 * reach), 0), max(int(box[1] - 3 * reach), 0)
    x1, y1 = min(int(box[2] + 3 * reach) + 1, side), min(int(box[3] + 3 * reach) + 1, side)
    px, py = np.meshgrid(np.arange(x0, x1) + 0.5, np.arange(y0, y1) + 0.5)
    cx, cy = (box[0] + box[2]) / 2, (box[1] + box[3]) / 2
    hx, hy = (box[2] - box[0]) / 2, (box[3] - box[1]) / 2
    fx, fy = px - cx, py - cy
    qx, qy = np.abs(fx) - hx + radius, np.abs(fy) - hy + radius
    distance = np.hypot(np.maximum(qx, 0), np.maximum(qy, 0)) + np.minimum(np.maximum(qx, qy), 0) - radius
    corner = (qx > 0) & (qy > 0)
    length = np.maximum(np.hypot(qx, qy), 1e-6)
    ox = np.where(corner, qx / length, np.where(qx > qy, 1.0, 0.0)) * np.where(fx >= 0, 1, -1)
    oy = np.where(corner, qy / length, np.where(qx > qy, 0.0, 1.0)) * np.where(fy >= 0, 1, -1)

    rim = np.clip(1 + distance / bezel, 0, 1)
    bend = rim**3
    mixed = (blur * (1 - 0.6 * bend))[..., None]

    def look(factor, channel):
        sx = np.clip((px - ox * bend * refraction * factor).astype(int), 0, side - 1)
        sy = np.clip((py - oy * bend * refraction * factor).astype(int), 0, side - 1)
        return (image[sy, sx] * (1 - mixed) + soft[sy, sx] * mixed)[..., channel]

    color = np.stack([look(1.1, 0), look(1.0, 1), look(0.9, 2)], axis=-1)
    if peak:
        # Coloured glass filters the light behind it instead of covering it.
        filtered = np.array(tint, dtype=np.float32) * (opacity + (peak - opacity) * np.clip(color * 4.0 - 0.25, 0, 1))
        color = color * 1.3 * CLEAR + filtered * (1 - CLEAR)
    else:
        color = color * 1.08 * (1 - opacity) + np.array(tint, dtype=np.float32) * opacity
    facing = (ox * -0.6 + oy * -0.8)[..., None]
    curve = (rim * rim)[..., None]
    color = color * (1 + 0.6 * curve * facing * depth) + 0.03 * curve * np.maximum(facing, 0) * depth
    hair = max(bezel * 0.09, 1.5)
    edge = np.clip((distance + 2.6 * hair) / (1.9 * hair), 0, 1)
    edge = (edge * edge * (3 - 2 * edge))[..., None]
    color = color + edge * (0.3 * np.maximum(facing, 0) ** 2 + 0.13 * np.maximum(-facing, 0) ** 2 + 0.035) * depth

    inside = np.clip(0.5 - distance, 0, 1)[..., None]
    shade = strength * np.exp(-np.maximum(distance, 0) / (max(reach, 1) * 0.4))[..., None] * (1 - inside)
    under = image[y0:y1, x0:x1]
    image[y0:y1, x0:x1] = np.clip(color * inside + under * (1 - inside) * (1 - shade), 0, 1)


def rgb(color):
    return [int(color[i : i + 2], 16) / 255 for i in (1, 3, 5)]


def draw(background, corner, colors, dot, scale=1.0):
    side = SIZE * OVERSAMPLE
    flame = Image.open(FLAME).convert("RGB").resize((side, side), Image.LANCZOS)
    if colors == WHITE:
        flame = ImageOps.grayscale(flame).convert("RGB")
    image = np.asarray(flame, dtype=np.float32) / 255

    dark = background == DARK
    radius = max(corner * OVERSAMPLE, 1)
    glass(image, (0, 0, side, side), radius, side * 0.07, side * 0.05, 0.45, 0.8 if dark else 0.88, 1.0, rgb(background), (0, 0))

    peak = 1.6 if dark and colors != WHITE else 1.0
    shapes, (dot_x, dot_r, dot_color), width, height = glyph(colors, dot)
    k = OVERSAMPLE * scale
    left, top = (side - width * k) / 2, (side - height * k) / 2
    for x, y, cell, tile_radius, color in shapes:
        box = (left + x * k, top + y * k, left + (x + cell) * k, top + (y + cell) * k)
        size = cell * k
        glass(image, box, tile_radius * k, size * 0.3, size * 0.34, 0.2, 0.62, 1.4, rgb(color), (size * 0.2, 0.45), peak=peak)

    out = Image.fromarray((image * 255).astype(np.uint8)).convert("RGBA")
    cx, cy = left + (dot_x + dot_r) * k, top + dot_r * k
    ImageDraw.Draw(out).ellipse((cx - dot_r * k, cy - dot_r * k, cx + dot_r * k, cy + dot_r * k), fill=dot_color)
    if corner:
        mask = Image.new("L", (side, side), 0)
        ImageDraw.Draw(mask).rounded_rectangle((0, 0, side - 1, side - 1), radius, fill=255)
        out.putalpha(mask)
    return out.resize((SIZE, SIZE), Image.LANCZOS)


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
    draw(LIGHT, 0, DEEP, DARK).convert("RGB").save(OUT / "ios-light-1024.png")
    # iOS turns the tinted icon to grey and tints it: white glyph on black, as Apple asks.
    draw("#000000", 0, WHITE, "#ffffff").convert("RGB").save(OUT / "ios-tinted-1024.png")

    # Android crops an adaptive icon's foreground to the middle two thirds, so the glyph is drawn smaller there.
    foreground = draw(DARK, 0, WARM, "#ffffff", scale=0.62)
    for density, legacy, adaptive in (("mdpi", 48, 108), ("hdpi", 72, 162), ("xhdpi", 96, 216), ("xxhdpi", 144, 324), ("xxxhdpi", 192, 432)):
        folder = ANDROID / f"mipmap-{density}"
        folder.mkdir(parents=True, exist_ok=True)
        icon.resize((legacy, legacy), Image.LANCZOS).save(folder / "ic_launcher.png")
        foreground.resize((adaptive, adaptive), Image.LANCZOS).save(folder / "ic_launcher_foreground.png")


if __name__ == "__main__":
    main()

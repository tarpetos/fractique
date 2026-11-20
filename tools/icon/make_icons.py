"""Draws the app icon and writes every size and format the platforms need.

Run from the repository root:  uv run --no-project --with pillow --with numpy python tools/icon/make_icons.py

The icon is a white lowercase f whose hook winds into a spiral, over a Buddhabrot. The Buddhabrot is the preset
buddhabrot-classic, kept as a scene beside this script and rendered by the command line of this repository:

    cargo build --release
    ./target/release/fractique render tools/icon/buddhabrot.json --size 1024x1024 --quality high -o tools/icon/buddhabrot.png

The script crops that picture to the figure, takes most of its color away and fades it into a near-black ground.
Below 64 pixels the letter is drawn heavier with a shorter spiral, and below 48 pixels it is also larger and the
Buddhabrot is blurred to a dim glow.

The other programs carry the same picture in a color of their own, with a sign beside the letter: a prompt on green
for the command line tool (cli), a four-pointed star on violet for the MCP server (mcp), a rack on amber for the HTTP
server (server). The Buddhabrot is tinted toward the color before the letter is laid on, so the letter stays white
to its edge and the three differ at 16 pixels by the color of the whole tile. PROGRAMS holds the three.
"""


from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, ImageFilter

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "assets" / "icon"
ANDROID = ROOT / "crates" / "fractique-app" / "android" / "res"
IOS = ROOT / "ios" / "Assets.xcassets" / "AppIcon.appiconset"
RENDER = Path(__file__).with_name("buddhabrot.png")

SIZE = 1024
OVERSAMPLE = 2
CORNER = 230
CROP = (82, 62, 942, 922)
GROUND = "#06080d"
FIT = (0.56, 0.79)
# From this many pixels up: the weight of the stroke, the turns of the spiral, the blur of the Buddhabrot, the growth.
DETAIL = [(64, 1.0, 1.45, 0, 1.0), (48, 1.1, 1.4, 0, 1.0), (24, 1.25, 1.1, 40, 1.05), (0, 1.4, 0.85, 60, 1.1)]
# Android shows the middle 72 of an adaptive icon's 108 units and promises only a circle of 66.
ADAPTIVE_FIT = (0.385, 0.535)
ADAPTIVE_REACH = 0.74
PROGRAMS = {"cli": "#3ddc84", "mcp": "#a78bfa", "server": "#ffb454"}
ICO = (16, 24, 32, 48, 64, 128, 256)
TINT = 0.6
# Beside its sign the letter stands left of the middle and a little smaller; the sign's side, left and top.
BESIDE = ((-0.1, 0.0), (0.9, 0.94))
SIGN = (0.42, 0.49, 0.48)


def rgb(color):
    return np.array([int(color[i : i + 2], 16) / 255 for i in (1, 3, 5)], dtype=np.float32)


def smooth(t):
    t = np.clip(t, 0, 1)
    return t * t * (3 - 2 * t)


def letter(weight, turns):
    eye_x, eye_y, reach, stem, foot, bar = 0.60, 0.36, 0.24, 0.36, 0.89, 0.625
    half = 0.046 * weight
    angle = np.linspace(0, turns * 2 * np.pi, 2500)
    radius = reach * np.exp(-0.24 * angle * angle / (angle + 1.2))
    points = np.stack([eye_x - radius * np.cos(angle), eye_y - radius * np.sin(angle)], axis=1)
    widths = half * (1 - 0.86 * (angle / angle[-1]) ** 0.8)
    thick = 0.027 * weight
    lean = thick * 0.28
    polygons = [
        [(stem - half, eye_y), (stem + half, eye_y), (stem + half, foot), (stem - half, foot)],
        [(stem - 0.0925 + lean, bar - thick), (stem + 0.1925 + lean, bar - thick), (stem + 0.1925 - lean, bar + thick), (stem - 0.0925 - lean, bar + thick)],
    ]
    return points, widths, polygons


def mark(weight, turns, fit, shift):
    points, widths, polygons = letter(weight, turns)
    corners = np.array([p for polygon in polygons for p in polygon])
    low = np.minimum((points - widths[:, None]).min(axis=0), corners.min(axis=0))
    high = np.maximum((points + widths[:, None]).max(axis=0), corners.max(axis=0))
    side = SIZE * OVERSAMPLE
    scale = min(fit[0] / (high[0] - low[0]), fit[1] / (high[1] - low[1])) * side
    offset = (np.array([0.5, 0.5]) + shift) * side - (low + high) / 2 * scale
    image = Image.new("L", (side, side), 0)
    pen = ImageDraw.Draw(image)
    for (x, y), r in zip(points * scale + offset, widths * scale):
        pen.ellipse((x - r, y - r, x + r, y + r), fill=255)
    for polygon in polygons:
        pen.polygon([tuple(np.array(p) * scale + offset) for p in polygon], fill=255)
    return image.resize((SIZE, SIZE), Image.LANCZOS)


def buddhabrot(reach, blur):
    side = round(SIZE * reach)
    figure = Image.open(RENDER).convert("RGB").crop(CROP).resize((side, side), Image.LANCZOS)
    if blur:
        figure = figure.filter(ImageFilter.GaussianBlur(blur)).point(lambda value: value * 0.5)
    figure = np.asarray(figure, dtype=np.float32) / 255
    y, x = np.mgrid[0:side, 0:side]
    distance = np.hypot((x + 0.5) / side - 0.5, (y + 0.5) / side - 0.5) * 2
    end = 1.5 if reach == 1 else 1.0
    figure = figure * smooth((end - distance) / (end - 0.6))[..., None]
    light = np.zeros((SIZE, SIZE, 3), dtype=np.float32)
    start = (SIZE - side) // 2
    light[start : start + side, start : start + side] = figure
    return light


def sign(program, side):
    box = round(SIZE * SIGN[0])
    big = box * OVERSAMPLE
    image = Image.new("L", (big, big), 0)
    pen = ImageDraw.Draw(image)
    width = round(big * 0.156)

    def stroke(points):
        points = [(x * big, y * big) for x, y in points]
        pen.line(points, fill=255, width=width, joint="curve")
        for x, y in points:
            pen.ellipse((x - width / 2, y - width / 2, x + width / 2, y + width / 2), fill=255)

    if program == "cli":
        stroke([(0.12, 0.2), (0.46, 0.5), (0.12, 0.8)])
        stroke([(0.58, 0.8), (0.9, 0.8)])
    elif program == "mcp":
        turn = np.pi / 4
        reach = [0.5 if point % 2 == 0 else 0.17 for point in range(8)]
        pen.polygon([(big / 2 + r * big * np.sin(turn * i), big / 2 - r * big * np.cos(turn * i)) for i, r in enumerate(reach)], fill=255)
    else:
        for row in range(3):
            top = 0.1 + row * 0.3
            pen.rounded_rectangle((0.06 * big, top * big, 0.94 * big, (top + 0.2) * big), 0.06 * big, fill=255)
            if side >= 96:
                pen.ellipse((0.14 * big, (top + 0.055) * big, 0.23 * big, (top + 0.145) * big), fill=0)
    placed = Image.new("L", (SIZE, SIZE), 0)
    placed.paste(image.resize((box, box), Image.LANCZOS), (round(SIZE * SIGN[1]), round(SIZE * SIGN[2])))
    return placed


def draw(side=SIZE, ground=GROUND, corner=CORNER, keep=0.45, gain=0.9, fit=FIT, reach=1.0, program=None):
    weight, turns, blur, growth = next(row[1:] for row in DETAIL if side >= row[0])
    shift, shrink = BESIDE if program else ((0.004, 0.0), (1.0, 1.0))
    letter_mask = mark(weight, turns, (fit[0] * growth * shrink[0], fit[1] * growth * shrink[1]), shift)
    shape = np.asarray(letter_mask, dtype=np.float32)[..., None] / 255
    shade = np.asarray(letter_mask.filter(ImageFilter.GaussianBlur(22)), dtype=np.float32)[..., None] / 255
    light = buddhabrot(reach, blur)
    grey = light.mean(axis=2, keepdims=True)
    image = rgb(ground) + (grey * (1 - keep) + light * keep) * gain
    if program:
        color = rgb(PROGRAMS[program])
        tinted = color * 0.1 + image.mean(axis=2, keepdims=True) * (color * 1.25 + 0.15)
        image = image * (1 - TINT) + tinted * TINT
    image = image * (1 - 0.6 * shade)
    if program:
        mask = sign(program, side)
        dark = np.clip(np.asarray(mask.filter(ImageFilter.GaussianBlur(36)), dtype=np.float32)[..., None] / 255 * 1.6, 0, 1)
        inked = np.asarray(mask, dtype=np.float32)[..., None] / 255
        image = (image * (1 - dark) + rgb(GROUND) * dark) * (1 - inked) + color * inked
    image = image * (1 - shape) + shape
    out = Image.fromarray((np.clip(image, 0, 1) * 255 + 0.5).astype(np.uint8)).convert("RGBA")
    if corner:
        big = SIZE * OVERSAMPLE
        mask = Image.new("L", (big, big), 0)
        ImageDraw.Draw(mask).rounded_rectangle((0, 0, big - 1, big - 1), corner * OVERSAMPLE, fill=255)
        out.putalpha(mask.resize((SIZE, SIZE), Image.LANCZOS))
    return out if side == SIZE else out.resize((side, side), Image.LANCZOS)


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    icons = {side: draw(side) for side in (16, 24, 32, 48, 64, 128, 256, 512, 1024)}
    for side, icon in icons.items():
        if side != 24:
            icon.save(OUT / f"icon-{side}.png")
    icons[256].save(OUT / "icon.ico", sizes=[(s, s) for s in ICO], append_images=[icons[s] for s in ICO[:-1]])
    for program in PROGRAMS:
        marked = {side: draw(side, program=program) for side in ICO}
        marked[64].save(OUT / f"{program}-64.png")
        marked[128].save(OUT / f"{program}-128.png")
        marked[256].save(OUT / f"{program}.ico", sizes=[(s, s) for s in ICO], append_images=[marked[s] for s in ICO[:-1]])
    icons[1024].save(OUT / "icon.icns", append_images=[icons[s] for s in (16, 32, 64, 128, 256, 512)])

    # iOS rounds the corners itself. It turns the tinted icon to grey and tints it: white on black, as Apple asks.
    draw(corner=0).convert("RGB").save(IOS / "light.png")
    draw(corner=0, gain=0.55).convert("RGB").save(IOS / "dark.png")
    draw(corner=0, ground="#000000", keep=0.0, gain=0.4).convert("RGB").save(IOS / "tinted.png")

    foreground = draw(corner=0, fit=ADAPTIVE_FIT, reach=ADAPTIVE_REACH)
    for density, legacy, adaptive in (("mdpi", 48, 108), ("hdpi", 72, 162), ("xhdpi", 96, 216), ("xxhdpi", 144, 324), ("xxxhdpi", 192, 432)):
        folder = ANDROID / f"mipmap-{density}"
        folder.mkdir(parents=True, exist_ok=True)
        draw(legacy).save(folder / "ic_launcher.png")
        foreground.resize((adaptive, adaptive), Image.LANCZOS).save(folder / "ic_launcher_foreground.png")
    color = f'<?xml version="1.0" encoding="utf-8"?>\n<resources>\n    <color name="ic_launcher_background">{GROUND}</color>\n</resources>\n'
    (ANDROID / "values" / "ic_launcher_background.xml").write_text(color)


if __name__ == "__main__":
    main()

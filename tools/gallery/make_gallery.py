import argparse
import json
import subprocess
import sys
import tempfile
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
GALLERY = ROOT / "docs" / "gallery"
SIZE = "960x540"
QUALITY = "high"
WEBP_QUALITIES = (84, 78, 72, 66)
LARGEST = 110_000
COLUMNS = 4

SECTIONS = [
    ("flames", "Flames", "Fractal flames, chosen from several thousand random scenes and their mutations.\n"),
    (
        "fractals",
        "Fractals",
        "Fractals of other kinds, most of them presets of their kind and some views of a random seed. They open in the\n"
        "**Fractals** tab.\n",
    ),
    (
        "wallpapers",
        "Wallpapers",
        "A wallpaper's file holds its seed, layout and material, and the shapes laid out for a 16:9 screen. The app\n"
        "opens it in the **Shapes** tab and lays the wallpaper out anew from the seed; the commands draw the shapes\n"
        "in the file. The first one is also {wallpaper}.\n",
    ),
]

FRONT = """# Gallery

{counts} made with Fractique, each a small picture and the `.frcq` file it was
rendered from. The file holds the whole scene, so it renders the same picture again at any size and quality, on any
machine.

To open one in the app, click its picture, download the `.frcq` file from the page that opens, and choose
**Scene**, **Open** in the app or double-click the file. To render one from the command line at 4K:

```sh
fractique render docs/gallery/{first} --size 3840x2160 --quality high -o {name}.png
```

The pictures are {size} pixels, rendered at `--quality {quality}` and saved as WebP. They and these pages are
made from the `.frcq` files by `tools/gallery/make_gallery.py` ([usage.md](../usage.md#14-the-gallery)).
"""


def run(binary, *arguments):
    done = subprocess.run([str(binary), *map(str, arguments)], capture_output=True, text=True)
    if done.returncode != 0:
        sys.exit(f"{' '.join(map(str, arguments))}: {done.stderr.strip()}")
    return done.stdout


def title_of(binary, source):
    return json.loads(run(binary, "convert", source, "--document"))["title"]


def draw(binary, source, folder):
    rendered = Path(folder) / "picture.png"
    run(binary, "render", source, "--size", SIZE, "--quality", QUALITY, "-o", rendered)
    target = source.with_suffix(".webp")
    picture = Image.open(rendered).convert("RGB")
    for quality in WEBP_QUALITIES:
        picture.save(target, "WEBP", quality=quality, method=6)
        if target.stat().st_size <= LARGEST:
            break
    return target


def wallpaper_command(binary, source):
    document = json.loads(run(binary, "convert", source, "--document"))
    wall = document["wall"]
    dark = " --dark" if wall["dark"] else ""
    return (
        f"`fractique render --wallpaper {document['seed']} --layout {wall['layout']} "
        f"--material {wall['material']}{dark} --size 3840x2160`"
    )


def grid(entries, prefix=""):
    lines = ["|" + " |" * COLUMNS, "|" + " --- |" * COLUMNS]
    for start in range(0, len(entries), COLUMNS):
        row = entries[start : start + COLUMNS]
        blank = [""] * (COLUMNS - len(row))
        pictures = [f"[![{title}]({prefix}{source.stem}.webp)]({prefix}{source.name})" for source, title in row]
        lines.append("| " + " | ".join(pictures + blank) + " |")
        lines.append("| " + " | ".join([title for _, title in row] + blank) + " |")
    return lines


def counted(parts):
    counts = [f"{len(found)} {title.lower()}" for _, title, _, found in parts]
    return ", ".join(counts[:-1]) + " and " + counts[-1] if len(counts) > 1 else counts[0]


def front(parts):
    first = parts[0][3][0][0]
    lines = [
        FRONT.format(
            counts=counted(parts)[0].upper() + counted(parts)[1:],
            first=f"{parts[0][0]}/{first.name}",
            name=first.stem,
            size=SIZE.replace("x", " × "),
            quality=QUALITY,
        )
    ]
    for folder, title, _, found in parts:
        lines.append(f"## [{title}]({folder}/README.md)\n")
        lines.extend(grid(found[:COLUMNS], f"{folder}/"))
        lines.append(f"\n[All {len(found)} {title.lower()}]({folder}/README.md)\n")
    return "\n".join(lines)


def section(binary, parts, at):
    folder, title, note, found = parts[at]
    after = parts[(at + 1) % len(parts)]
    ways = f"[Gallery](../README.md) · next: [{after[1]}](../{after[0]}/README.md)\n"
    lines = [f"# {title}\n", ways]
    lines.append(note.format(wallpaper=wallpaper_command(binary, found[0][0]) if "{wallpaper}" in note else ""))
    lines.extend(grid(found))
    lines.append("\n" + ways)
    return "\n".join(lines)


def main():
    program = "fractique.exe" if sys.platform == "win32" else "fractique"
    parser = argparse.ArgumentParser(description="Rebuild the gallery's pictures and its pages from the .frcq files")
    parser.add_argument("--fractique", type=Path, default=ROOT / "target" / "release" / program)
    parser.add_argument("--page-only", action="store_true")
    parser.add_argument("names", nargs="*", help="only these, by the start of the file name: 007 012 F W")
    options = parser.parse_args()

    parts = []
    for folder, title, note in SECTIONS:
        sources = sorted((GALLERY / folder).glob("*.frcq"))
        if sources:
            parts.append((folder, title, note, [(source, title_of(options.fractique, source)) for source in sources]))
    if not parts:
        sys.exit(f"no .frcq files in the folders of {GALLERY}")
    if not options.page_only:
        every = [source for part in parts for source, _ in part[3]]
        chosen = [s for s in every if not options.names or any(s.name.startswith(n) for n in options.names)]
        with tempfile.TemporaryDirectory() as folder:
            for at, source in enumerate(chosen, 1):
                target = draw(options.fractique, source, folder)
                print(f"{at}/{len(chosen)} {target.name} {target.stat().st_size // 1024} KB", flush=True)
    (GALLERY / "README.md").write_text(front(parts), encoding="utf-8", newline="\n")
    for at, part in enumerate(parts):
        (GALLERY / part[0] / "README.md").write_text(section(options.fractique, parts, at), encoding="utf-8", newline="\n")
    total = sum(path.stat().st_size for path in GALLERY.rglob("*") if path.is_file())
    print(f"{GALLERY.relative_to(ROOT)}: {counted(parts)}, {total / 1e6:.1f} MB")


if __name__ == "__main__":
    main()

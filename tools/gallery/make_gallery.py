import argparse
import json
import subprocess
import sys
import tempfile
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
GALLERY = ROOT / "docs" / "gallery"
PAGE = ROOT / "docs" / "gallery.md"
SIZE = "960x540"
QUALITY = "high"
WEBP_QUALITY = 84
COLUMNS = 4

INTRO = """# Gallery

One hundred flames made with Fractique, each a small picture and the `.frcq` file it was rendered from. The file
holds the whole scene, so it renders the same picture again at any size and quality, on any machine.

To open one in the app, click its picture, download the `.frcq` file from the page that opens, and choose
**Scene**, **Open** in the app or double-click the file. To render one from the command line at 4K:

```sh
fractique render docs/gallery/{first}.frcq --size 3840x2160 --quality high -o {first}.png
```

The pictures here are {size} pixels, rendered at `--quality {quality}` and saved as WebP. They are made from the
`.frcq` files by `tools/gallery/make_gallery.py` ([usage.md](usage.md#14-the-gallery)).
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
    Image.open(rendered).convert("RGB").save(target, "WEBP", quality=WEBP_QUALITY, method=6)
    return target


def page(entries):
    lines = [INTRO.format(first=entries[0][0].stem, size=SIZE.replace("x", " × "), quality=QUALITY)]
    lines.append("|" + " |" * COLUMNS)
    lines.append("|" + " --- |" * COLUMNS)
    for start in range(0, len(entries), COLUMNS):
        row = entries[start : start + COLUMNS]
        blank = [""] * (COLUMNS - len(row))
        pictures = [f"[![{title}](gallery/{source.stem}.webp)](gallery/{source.name})" for source, title in row]
        lines.append("| " + " | ".join(pictures + blank) + " |")
        lines.append("| " + " | ".join([title for _, title in row] + blank) + " |")
    return "\n".join(lines) + "\n"


def main():
    program = "fractique.exe" if sys.platform == "win32" else "fractique"
    parser = argparse.ArgumentParser(description="Rebuild the gallery's pictures and its page from the .frcq files")
    parser.add_argument("--fractique", type=Path, default=ROOT / "target" / "release" / program)
    parser.add_argument("--page-only", action="store_true")
    parser.add_argument("names", nargs="*", help="only these, by the start of the file name: 007 012")
    options = parser.parse_args()

    sources = sorted(GALLERY.glob("*.frcq"))
    if not sources:
        sys.exit(f"no .frcq files in {GALLERY}")
    entries = [(source, title_of(options.fractique, source)) for source in sources]
    if not options.page_only:
        chosen = [s for s in sources if not options.names or any(s.name.startswith(n) for n in options.names)]
        with tempfile.TemporaryDirectory() as folder:
            for at, source in enumerate(chosen, 1):
                target = draw(options.fractique, source, folder)
                print(f"{at}/{len(chosen)} {target.name} {target.stat().st_size // 1024} KB", flush=True)
    PAGE.write_text(page(entries), encoding="utf-8", newline="\n")
    total = sum(path.stat().st_size for path in GALLERY.iterdir())
    print(f"{PAGE.relative_to(ROOT)}: {len(entries)} flames, {GALLERY.relative_to(ROOT)} is {total / 1e6:.1f} MB")


if __name__ == "__main__":
    main()

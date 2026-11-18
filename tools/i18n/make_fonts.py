"""Cuts the fallback fonts down to the characters the app's texts use.

The app carries Inter for Latin and Cyrillic. Japanese, Chinese, Korean and Hindi need other fonts, which are
megabytes each in full; the app only ever shows the few characters of its own texts.

Run from the repository root:  uv run --no-project --with fonttools python tools/i18n/make_fonts.py
The full fonts are downloaded once into target/fonts-src.
"""

import json
import urllib.request
from pathlib import Path

from fontTools import subset
from fontTools.ttLib import TTFont
from fontTools.varLib import instancer

ROOT = Path(__file__).resolve().parents[2]
LOCALES = ROOT / "crates" / "fractique-app" / "locales"
OUT = ROOT / "crates" / "fractique-app" / "assets" / "fonts"
CACHE = ROOT / "target" / "fonts-src"
SOURCE = "https://github.com/google/fonts/raw/main/ofl"

FONTS = {
    "ja": ("notosansjp/NotoSansJP%5Bwght%5D.ttf", "NotoSansJP-subset.ttf", "日本語"),
    "zh-Hans": ("notosanssc/NotoSansSC%5Bwght%5D.ttf", "NotoSansSC-subset.ttf", "简体中文"),
    "ko": ("notosanskr/NotoSansKR%5Bwght%5D.ttf", "NotoSansKR-subset.ttf", "한국어"),
    "hi": ("notosansdevanagari/NotoSansDevanagari%5Bwdth%2Cwght%5D.ttf", "NotoSansDevanagari-subset.ttf", "हिन्दी"),
}


def main():
    CACHE.mkdir(parents=True, exist_ok=True)
    for code, (path, name, own_name) in FONTS.items():
        full = CACHE / name.replace("-subset", "")
        if not full.exists():
            urllib.request.urlretrieve(f"{SOURCE}/{path}", full)
        texts = json.loads((LOCALES / f"{code}.json").read_text(encoding="utf-8"))
        families = [json.loads(path.read_text(encoding="utf-8")) for path in sorted((LOCALES / "fractals").glob("*.json"))]
        fractals = "".join(names[code] for family in families for names in family.values())
        characters = sorted(set("".join(texts.values()) + fractals + own_name) - set(" "))

        font = TTFont(full)
        axes = {axis.axisTag: axis.defaultValue for axis in font["fvar"].axes}
        axes["wght"] = 500
        font = instancer.instantiateVariableFont(font, axes)
        options = subset.Options(layout_features=["*"], name_IDs=[1, 2], notdef_outline=True)
        cutter = subset.Subsetter(options)
        cutter.populate(text="".join(characters))
        cutter.subset(font)
        font.save(OUT / name)
        print(f"{name}: {len(characters)} characters, {(OUT / name).stat().st_size} bytes")


if __name__ == "__main__":
    main()

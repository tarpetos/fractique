# Gallery

100 flames, 100 fractals and 100 wallpapers made with Fractique, each a small picture and the `.frcq` file it was
rendered from. The file holds the whole scene, so it renders the same picture again at any size and quality, on any
machine.

To open one in the app, click its picture, download the `.frcq` file from the page that opens, and choose
**Scene**, **Open** in the app or double-click the file. To render one from the command line at 4K:

```sh
fractique render docs/gallery/flames/001-phoenix-flare.frcq --size 3840x2160 --quality high -o 001-phoenix-flare.png
```

The pictures are 960 × 540 pixels, rendered at `--quality high` and saved as WebP. They and these pages are
made from the `.frcq` files by `tools/gallery/make_gallery.py` ([usage.md](../usage.md#14-the-gallery)).

## [Flames](flames/README.md)

| | | | |
| --- | --- | --- | --- |
| [![Phoenix Flare](flames/001-phoenix-flare.webp)](flames/001-phoenix-flare.frcq) | [![Peacock Plume](flames/002-peacock-plume.webp)](flames/002-peacock-plume.frcq) | [![Rose Heart](flames/003-rose-heart.webp)](flames/003-rose-heart.frcq) | [![Nautilus Frost](flames/004-nautilus-frost.webp)](flames/004-nautilus-frost.frcq) |
| Phoenix Flare | Peacock Plume | Rose Heart | Nautilus Frost |

[All 100 flames](flames/README.md)

## [Fractals](fractals/README.md)

| | | | |
| --- | --- | --- | --- |
| [![Ember Cathedral](fractals/F001-ember-cathedral.webp)](fractals/F001-ember-cathedral.frcq) | [![Teal Maelstrom](fractals/F002-teal-maelstrom.webp)](fractals/F002-teal-maelstrom.frcq) | [![Golden Galaxies](fractals/F003-golden-galaxies.webp)](fractals/F003-golden-galaxies.frcq) | [![Neon Garden](fractals/F004-neon-garden.webp)](fractals/F004-neon-garden.frcq) |
| Ember Cathedral | Teal Maelstrom | Golden Galaxies | Neon Garden |

[All 100 fractals](fractals/README.md)

## [Wallpapers](wallpapers/README.md)

| | | | |
| --- | --- | --- | --- |
| [![Pink Lotus](wallpapers/W001-pink-lotus.webp)](wallpapers/W001-pink-lotus.frcq) | [![Teal Ribbons](wallpapers/W002-teal-ribbons.webp)](wallpapers/W002-teal-ribbons.frcq) | [![Aqua Fan](wallpapers/W003-aqua-fan.webp)](wallpapers/W003-aqua-fan.frcq) | [![Ember Discs](wallpapers/W004-ember-discs.webp)](wallpapers/W004-ember-discs.frcq) |
| Pink Lotus | Teal Ribbons | Aqua Fan | Ember Discs |

[All 100 wallpapers](wallpapers/README.md)

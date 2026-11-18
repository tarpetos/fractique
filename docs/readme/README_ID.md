<div align="center">
  <img src="../../assets/icon/icon-256.png" alt="Fractique" width="128" height="128">
  <h1>Fractique</h1>
  <p><strong>Generator api fraktal yang menghasilkan gambar yang sama dari masukan yang sama, di platform mana pun.</strong></p>
  <p><sub>
    <a href="../../README.md">English</a> · <a href="README_UK.md">Українська</a> · <a href="README_RU.md">Русский</a> · <a href="README_DE.md">Deutsch</a> · <a href="README_FR.md">Français</a> · <a href="README_ES.md">Español</a> · <a href="README_IT.md">Italiano</a> · <a href="README_JA.md">日本語</a><br>
    <a href="README_CN.md">简体中文</a> · <a href="README_PT.md">Português</a> · <a href="README_KO.md">한국어</a> · <a href="README_HI.md">हिन्दी</a> · <a href="README_TR.md">Türkçe</a> · <a href="README_PL.md">Polski</a> · <strong>Bahasa Indonesia</strong>
  </sub></p>
</div>

Fractique menggambar api fraktal: fraktal yang berpendar dan halus, terbuat dari kurva tipis dan cahaya lembut. Anda memilih preset, membuat adegan acak, atau menumpuk beberapa api, lalu merendernya pada resolusi berapa pun, dari layar ponsel sampai wallpaper 4K. Sebuah adegan adalah satu berkas JSON kecil, dan berkas yang sama selalu menghasilkan piksel yang persis sama di Windows, macOS, Linux, Android, dan iOS.

<p align="center">
  <img src="../samples/aurora-spiral.png" alt="Preset aurora-spiral: untaian spiral hijau kebiruan dan hijau di latar gelap" width="880">
</p>

## Yang sudah berfungsi

- **Perender** (`fractique-core`): api fraktal dengan 49 variasi, lapisan dengan mode campur, simetri putar dan cermin, estimasi kepadatan, pendar, resolusi apa pun.
- **Aplikasi:** satu basis kode untuk Windows, Linux, Android, iOS, dan web, dengan panel kaca di atas api, dalam 15 bahasa.
- **Preset:** 9 adegan bawaan.
- **Generator acak:** setiap benih adalah satu adegan; adegan yang hanya akan menjadi titik atau persegi penuh dilewati.
- **Mutasi:** varian dari adegan yang Anda sukai, dengan kekuatan dari 0 sampai 1.
- **Pengulangan yang persis sama:** adegan yang sama menghasilkan piksel yang sama di setiap mesin, diperiksa oleh pengujian di Linux, Windows, dan macOS ([docs/determinism.md](../determinism.md)).
- **Impor:** berkas `.flame` dari flam3 dan Apophysis diubah menjadi adegan.
- **Alat baris perintah** (`fractique`): merender adegan menjadi PNG.
- **Server MCP** (`fractique mcp`): agen AI membuat, mengubah, dan merender adegan ([docs/mcp.md](../mcp.md)).

| | |
| --- | --- |
| ![emerald-star](../samples/emerald-star.png) | ![ember-vortex](../samples/ember-vortex.png) |
| `emerald-star` (dua lapisan) | `ember-vortex` |

## Penyiapan

Satu perintah membangun semuanya. Rust dipasang jika belum ada.

| Sistem | Perintah |
| --- | --- |
| Linux, macOS | `./setup.sh` |
| Windows | `setup.cmd` |

Kemudian:

```sh
# sebuah preset sebagai wallpaper 4K
./target/release/fractique render --preset aurora-spiral --size 3840x2160 --quality high -o aurora.png

# adegan acak dari benih 42, untuk layar ponsel
./target/release/fractique render --random 42 --size 1080x2400 --quality mobile -o phone.png

# simpan adegan acak sebagai berkas, ubah, lalu render
./target/release/fractique randomize 42 -o scene.json
./target/release/fractique render scene.json -o mine.png
```

Di Windows alatnya ada di `target\release\fractique.exe`. Semua perintah dan opsi: [docs/usage.md](../usage.md).

## Dokumentasi

Dokumentasi ditulis dalam bahasa Inggris.

| | |
| --- | --- |
| [docs/usage.md](../usage.md) | Perintah, tingkat kualitas, cara menyunting adegan |
| [docs/scene-format.md](../scene-format.md) | Setiap kolom berkas adegan, setiap variasi |
| [docs/mcp.md](../mcp.md) | Server MCP: menghubungkan agen AI, alat-alatnya |
| [docs/ios.md](../ios.md) | Membangun dan menjalankan di iPhone atau di Simulator iOS |
| [docs/architecture.md](../architecture.md) | Bagaimana bagian-bagiannya saling terhubung, bagaimana render bekerja |
| [docs/determinism.md](../determinism.md) | Mengapa render berulang persis sama, apa yang dijanjikan dan apa yang tidak |
| [docs/performance.md](../performance.md) | Waktu render dan memori hasil pengukuran |
| [docs/roadmap.md](../roadmap.md) | Apa yang sudah selesai dan apa berikutnya |

## Lisensi

[PolyForm Noncommercial License 1.0.0](../../LICENSE.md): gratis untuk penggunaan nonkomersial apa pun. Untuk penggunaan komersial, mintalah lisensi terpisah kepada penulis. Lisensi pihak ketiga tercantum di [THIRD_PARTY_NOTICES.md](../../THIRD_PARTY_NOTICES.md). Kontribusi: lihat [CONTRIBUTING.md](../../CONTRIBUTING.md).

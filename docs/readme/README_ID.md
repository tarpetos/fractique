<div align="center">
  <img src="../../assets/icon/icon-256.png" alt="Fractique" width="128" height="128">
  <h1>Fractique</h1>
  <p><strong>Generator fractal flame dan wallpaper yang menghasilkan gambar yang sama dari masukan yang sama, di semua platform.</strong></p>
  <p><sub>
    <a href="../../README.md">English</a> · <a href="README_UK.md">Українська</a> · <a href="README_RU.md">Русский</a> · <a href="README_DE.md">Deutsch</a> · <a href="README_FR.md">Français</a> · <a href="README_ES.md">Español</a> · <a href="README_IT.md">Italiano</a> · <a href="README_JA.md">日本語</a><br>
    <a href="README_CN.md">简体中文</a> · <a href="README_PT.md">Português</a> · <a href="README_KO.md">한국어</a> · <a href="README_HI.md">हिन्दी</a> · <a href="README_TR.md">Türkçe</a> · <a href="README_PL.md">Polski</a> · <strong>Bahasa Indonesia</strong>
  </sub></p>
</div>

Fractique membuat dua jenis gambar. Fractal flame: fraktal bercahaya dan halus yang terbentuk dari kurva tipis dan cahaya lembut. Dan wallpaper: lembaran datar dengan tepi bercahaya dan bayangan lembut, masing-masing dalam versi terang dan gelap. Anda memilih preset, membuat gambar acak, atau menumpuk beberapa flame, lalu mengekspornya dalam ukuran apa pun, dari layar ponsel hingga monitor 4K. Sebuah scene adalah satu berkas kecil, dan berkas yang sama selalu menghasilkan piksel yang persis sama, di Windows, macOS, Linux, Android, dan iOS.

<p align="center">
  <img src="../samples/aurora-spiral.png" alt="Preset aurora-spiral: untaian spiral toska dan hijau di atas latar gelap" width="880">
</p>

## Yang dilakukan Fractique

- **Flame:** 49 variasi, layer dengan mode blend, simetri putar dan cermin, estimasi kerapatan, glow, dan 29 preset bawaan.
- **Wallpaper:** satu untuk setiap seed, dari 14 tata letak dan 6 material, masing-masing dalam versi terang dan gelap, ditata sesuai ukuran layar.
- **Aplikasi:** satu basis kode untuk Windows, macOS, Linux, Android, iOS, dan web. Gambar memenuhi jendela dan panel kaca melayang di atasnya; di desktop aplikasi memiliki bilah judul kaca sendiri. Dalam 15 bahasa.
- **Randomize dan Mutate:** setiap seed adalah sebuah gambar; flame yang hanya akan menjadi titik atau kotak penuh dilewati. Mutate membuat varian dari gambar yang Anda sukai.
- **Layer:** tambahkan flame dari preset atau flame acak, hapus, ubah urutannya, dan atur blend (add, screen, over) serta opasitas masing-masing. Hingga 16 layer.
- **Ukuran gambar:** ukuran jendela itu sendiri, salah satu dari 21 layar (komputer hingga 4K, ponsel dan tablet Android, iPhone dan iPad), atau lebar dan tinggi berapa pun dari 16 hingga 16384 piksel.
- **Zoom dalam:** seret, roda mouse, dan cubit menggeser tampilan dan memperbesarnya hingga sepuluh miliar kali. Gambar langsung mengikuti dan digambar ulang dengan tajam saat tampilan berhenti.
- **Ponsel:** layar tersendiri. Satu tombol bulat membuat gambar baru, usapan berpindah ke wallpaper berikutnya, sebuah lembar berisi gambar kecil yang hidup untuk memilih gaya, dan gambar yang Anda simpan menjadi koleksi.
- **Ekspor:** PNG dalam ukuran apa pun, dengan tiga tingkat kualitas, dirender di latar belakang. Di ponsel, gambar disimpan ke Foto atau galeri, atau dibagikan.
- **Berkas proyek:** berkas `.frcq` menyimpan scene beserta tampilan, tone, dan ukurannya, sehingga terbuka persis seperti saat disimpan ([docs/scene-format.md](../scene-format.md)). Scene `.json` biasa dan berkas `.flame` dari flam3 dan Apophysis juga dapat dibuka. Saat dijalankan lagi, aplikasi kembali ke sesi terakhir.
- **Pembaruan:** sebuah jendela menampilkan setiap versi yang dirilis beserta catatannya dan memasang versi mana pun. Di Windows aplikasinya adalah satu `.exe` portabel yang mengganti dirinya sendiri.
- **Keterulangan yang persis:** scene yang sama menghasilkan piksel yang sama di semua mesin, diperiksa oleh pengujian di Linux, Windows, dan macOS ([docs/determinism.md](../determinism.md)).
- **Alat baris perintah** (`fractique`): merender flame dan wallpaper ke PNG, membuat scene acak dan varian, mengonversi berkas `.flame`.
- **Server MCP** (`fractique mcp`): agen AI membuat, mengubah, dan merender scene ([docs/mcp.md](../mcp.md)).
- **Server HTTP** (`fractique-server`): API HTTP dengan antrean render dan batasan, MCP untuk agen AI di `/mcp`, menyajikan aplikasi web, tersedia sebagai image Docker ([docs/server.md](../server.md)).

| | |
| --- | --- |
| ![emerald-star](../samples/emerald-star.png) | ![ember-vortex](../samples/ember-vortex.png) |
| `emerald-star` (dua layer) | `ember-vortex` |
| ![golden-bloom](../samples/golden-bloom.png) | ![solar-mandala](../samples/solar-mandala.png) |
| `golden-bloom` | `solar-mandala` |

Wallpaper, terang dan gelap, masing-masing dengan tata letak dan materialnya:

| | | | |
| --- | --- | --- | --- |
| ![waves](../samples/wallpaper-waves-light.png) | ![burst](../samples/wallpaper-burst-light.png) | ![tiles](../samples/wallpaper-tiles-light.png) | ![fan](../samples/wallpaper-fan-light.png) |
| `waves`, `glass` | `burst`, `glow` | `tiles`, `relief` | `fan`, `glass` |
| ![discs](../samples/wallpaper-discs-dark.png) | ![bars](../samples/wallpaper-bars-dark.png) | ![arcs](../samples/wallpaper-arcs-dark.png) | ![lines](../samples/wallpaper-lines-dark.png) |
| `discs`, `paper` | `bars`, `paper` | `arcs`, `carbon` | `lines`, `paper` |

## Mulai cepat

Satu perintah membangun aplikasi dan alat baris perintah. Rust dipasang jika belum ada.

| Sistem | Perintah |
| --- | --- |
| Linux, macOS | `./setup.sh` |
| Windows | `setup.cmd` |

Jalankan aplikasi:

```sh
./target/release/fractique-gui
```

Atau render dari baris perintah:

```sh
# sebuah preset sebagai gambar 4K
./target/release/fractique render --preset aurora-spiral --size 3840x2160 --quality high -o aurora.png

# flame acak dari seed 42, untuk layar ponsel
./target/release/fractique render --random 42 --size 1080x2400 --quality mobile -o phone.png

# wallpaper dari seed 7 untuk ponsel, terang dan gelap
./target/release/fractique render --wallpaper 7 --size 1290x2796 -o wallpaper.png
./target/release/fractique render --wallpaper 7 --dark --size 1290x2796 -o wallpaper-dark.png

# simpan scene acak sebagai berkas, ubah, lalu render
./target/release/fractique randomize 42 -o scene.json
./target/release/fractique render scene.json -o mine.png
```

Di Windows programnya adalah `target\release\fractique-gui.exe` dan `target\release\fractique.exe`. Semua yang dilakukan aplikasi dan perintahnya: [docs/usage.md](../usage.md).

## Dokumentasi

Dokumentasi ditulis dalam bahasa Inggris.

| | |
| --- | --- |
| [docs/usage.md](../usage.md) | Aplikasi, perintah, tingkat kualitas, cara menyunting scene |
| [docs/scene-format.md](../scene-format.md) | Setiap field berkas scene, setiap variasi, lembaran, berkas `.frcq` |
| [docs/mcp.md](../mcp.md) | Server MCP: menghubungkan agen AI, daftar alat |
| [docs/server.md](../server.md) | Server HTTP: endpoint, batasan, Docker |
| [docs/ios.md](../ios.md) | Membangun dan menjalankan di iPhone atau di iOS Simulator |
| [docs/releasing.md](../releasing.md) | Cara rilis dibuat, cara aplikasi memperbarui dirinya |
| [docs/architecture.md](../architecture.md) | Bagaimana bagian-bagiannya saling terhubung, cara kerja render |
| [docs/determinism.md](../determinism.md) | Mengapa render terulang persis, apa yang dijanjikan dan apa yang tidak |
| [docs/performance.md](../performance.md) | Waktu render dan memori hasil pengukuran |
| [docs/roadmap.md](../roadmap.md) | Apa yang sudah selesai dan apa berikutnya |

## Lisensi

[PolyForm Noncommercial License 1.0.0](../../LICENSE.md): gratis untuk penggunaan nonkomersial apa pun. Untuk penggunaan komersial, mintalah lisensi terpisah kepada penulis. Lisensi pihak ketiga tercantum di [THIRD_PARTY_NOTICES.md](../../THIRD_PARTY_NOTICES.md). Kontribusi: lihat [CONTRIBUTING.md](../../CONTRIBUTING.md).

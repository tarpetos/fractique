<div align="center">
  <img src="../../assets/icon/icon-256.png" alt="Fractique" width="128" height="128">
  <h1>Fractique</h1>
  <p><strong>Aynı girdiden her platformda aynı görüntüyü üreten bir fraktal alev üreteci.</strong></p>
  <p><sub>
    <a href="../../README.md">English</a> · <a href="README_UK.md">Українська</a> · <a href="README_RU.md">Русский</a> · <a href="README_DE.md">Deutsch</a> · <a href="README_FR.md">Français</a> · <a href="README_ES.md">Español</a> · <a href="README_IT.md">Italiano</a> · <a href="README_JA.md">日本語</a><br>
    <a href="README_CN.md">简体中文</a> · <a href="README_PT.md">Português</a> · <a href="README_KO.md">한국어</a> · <a href="README_HI.md">हिन्दी</a> · <strong>Türkçe</strong> · <a href="README_PL.md">Polski</a> · <a href="README_ID.md">Bahasa Indonesia</a>
  </sub></p>
</div>

Fractique fraktal alevler çizer: ince eğrilerden ve yumuşak ışıktan oluşan, parlayan, tül gibi fraktallar. Bir hazır ayar seçer, rastgele bir sahne üretir ya da birkaç alevi üst üste koyarsınız ve sonucu telefon ekranından 4K duvar kağıdına kadar istediğiniz çözünürlükte oluşturursunuz. Bir sahne küçük bir JSON dosyasıdır ve aynı dosya Windows, macOS, Linux, Android ve iOS üzerinde her zaman tam olarak aynı pikselleri verir.

<p align="center">
  <img src="../samples/aurora-spiral.png" alt="aurora-spiral hazır ayarı: koyu arka planda turkuaz ve yeşil sarmal ışık telleri" width="880">
</p>

## Bugün çalışanlar

- **Oluşturucu** (`fractique-core`): 28 varyasyonlu fraktal alevler, karışım modlu katmanlar, dönme ve ayna simetrisi, yoğunluk kestirimi, ışıltı, her çözünürlük.
- **Uygulama:** Windows, Linux, Android, iOS ve web için tek kod tabanı, alevin üzerinde cam paneller, 15 dilde.
- **Hazır ayarlar:** 9 yerleşik sahne.
- **Rastgele üretici:** her tohum bir sahnedir; yalnızca bir nokta ya da dolu bir kare olacak sahneler atlanır.
- **Mutasyon:** beğendiğiniz bir sahnenin 0 ile 1 arasında bir güçle üretilen çeşitlemesi.
- **Tam yinelenebilirlik:** aynı sahne her makinede aynı pikselleri verir; bunu Linux, Windows ve macOS üzerindeki testler doğrular ([docs/determinism.md](../determinism.md)).
- **İçe aktarma:** flam3 ve Apophysis `.flame` dosyaları sahnelere dönüştürülür.
- **Komut satırı aracı** (`fractique`): sahneleri PNG olarak oluşturur.
- **MCP sunucusu** (`fractique mcp`): bir yapay zekâ ajanı sahneler oluşturur, değiştirir ve görüntüye dönüştürür ([docs/mcp.md](../mcp.md)).
- **HTTP sunucusu** (`fractique-server`): işleme kuyruğu ve sınırları olan bir HTTP API’si; web uygulamasını sunar, Docker imajı olarak gelir ([docs/server.md](../server.md)).

| | |
| --- | --- |
| ![emerald-star](../samples/emerald-star.png) | ![ember-vortex](../samples/ember-vortex.png) |
| `emerald-star` (iki katman) | `ember-vortex` |

## Kurulum

Tek bir komut her şeyi derler. Rust yoksa kurar.

| Sistem | Komut |
| --- | --- |
| Linux, macOS | `./setup.sh` |
| Windows | `setup.cmd` |

Ardından:

```sh
# 4K duvar kağıdı olarak bir hazır ayar
./target/release/fractique render --preset aurora-spiral --size 3840x2160 --quality high -o aurora.png

# 42 tohumunun rastgele sahnesi, telefon ekranı için
./target/release/fractique render --random 42 --size 1080x2400 --quality mobile -o phone.png

# rastgele bir sahneyi dosya olarak sakla, değiştir, oluştur
./target/release/fractique randomize 42 -o scene.json
./target/release/fractique render scene.json -o mine.png
```

Windows’ta araç `target\release\fractique.exe` konumundadır. Tüm komutlar ve seçenekler: [docs/usage.md](../usage.md).

## Belgeler

Belgeler İngilizcedir.

| | |
| --- | --- |
| [docs/usage.md](../usage.md) | Komutlar, kalite düzeyleri, bir sahnenin nasıl düzenleneceği |
| [docs/scene-format.md](../scene-format.md) | Sahne dosyasının her alanı, her varyasyon |
| [docs/mcp.md](../mcp.md) | MCP sunucusu: bir yapay zekâ ajanını bağlama, araçlar |
| [docs/server.md](../server.md) | HTTP sunucusu: uç noktalar, sınırlar, Docker |
| [docs/ios.md](../ios.md) | iPhone’da ya da iOS Simülatöründe derleme ve çalıştırma |
| [docs/architecture.md](../architecture.md) | Parçaların birbirine nasıl oturduğu, bir oluşturmanın nasıl işlediği |
| [docs/determinism.md](../determinism.md) | Bir oluşturmanın neden tam olarak yinelendiği, nelerin vaat edilip nelerin edilmediği |
| [docs/performance.md](../performance.md) | Ölçülen oluşturma süreleri ve bellek |
| [docs/roadmap.md](../roadmap.md) | Neler bitti, sırada ne var |

## Lisans

[PolyForm Noncommercial License 1.0.0](../../LICENSE.md): ticari olmayan her kullanım için ücretsizdir. Ticari kullanım için yazardan ayrı bir lisans isteyin. Üçüncü taraf lisansları [THIRD_PARTY_NOTICES.md](../../THIRD_PARTY_NOTICES.md) dosyasında listelenmiştir. Katkılar: [CONTRIBUTING.md](../../CONTRIBUTING.md) dosyasına bakın.

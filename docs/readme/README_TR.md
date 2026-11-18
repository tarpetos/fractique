<div align="center">
  <img src="../../assets/icon/icon-256.png" alt="Fractique" width="128" height="128">
  <h1>Fractique</h1>
  <p><strong>Aynı girdiden her platformda aynı görüntüyü üreten bir fraktal alev ve duvar kâğıdı üreteci.</strong></p>
  <p><sub>
    <a href="../../README.md">English</a> · <a href="README_UK.md">Українська</a> · <a href="README_RU.md">Русский</a> · <a href="README_DE.md">Deutsch</a> · <a href="README_FR.md">Français</a> · <a href="README_ES.md">Español</a> · <a href="README_IT.md">Italiano</a> · <a href="README_JA.md">日本語</a><br>
    <a href="README_CN.md">简体中文</a> · <a href="README_PT.md">Português</a> · <a href="README_KO.md">한국어</a> · <a href="README_HI.md">हिन्दी</a> · <strong>Türkçe</strong> · <a href="README_PL.md">Polski</a> · <a href="README_ID.md">Bahasa Indonesia</a>
  </sub></p>
</div>

Fractique iki tür görüntü üretir. Fraktal alevler: ince eğrilerden ve yumuşak ışıktan oluşan, parlayan, tül gibi fraktallar. Ve duvar kâğıtları: kenarları aydınlatılmış, yumuşak gölgeli düz levhalar; her birinin açık ve koyu sürümü vardır. Bir ön ayar seçer, rastgele bir görüntü üretir ya da birkaç alevi üst üste bindirirsiniz ve sonucu telefon ekranından 4K monitöre kadar istediğiniz boyutta dışa aktarırsınız. Bir sahne tek bir küçük dosyadır ve aynı dosya Windows, macOS, Linux, Android ve iOS’ta her zaman tam olarak aynı pikselleri verir.

<p align="center">
  <img src="../samples/aurora-spiral.png" alt="aurora-spiral ön ayarı: koyu bir arka planda turkuaz ve yeşil sarmal tüller" width="880">
</p>

## Fractique neler yapar

- **Alevler:** 49 varyasyon, karıştırma modlu katmanlar, dönme ve ayna simetrisi, yoğunluk kestirimi, parıltı ve 29 yerleşik ön ayar.
- **Duvar kâğıtları:** her tohum için bir tane; 14 düzen ve 6 malzemeden, her biri açık ve koyu sürümüyle, ekranın boyutuna göre yerleştirilir.
- **Uygulama:** Windows, macOS, Linux, Android, iOS ve web için tek kod tabanı. Görüntü pencereyi doldurur, üzerinde cam paneller süzülür; masaüstünde uygulamanın kendi cam başlık çubuğu vardır. 15 dilde.
- **Randomize ve Mutate:** her tohum bir görüntüdür; bir nokta ya da dolu bir kare olacak alevler atlanır. Mutate, beğendiğinizin bir varyantını üretir.
- **Katmanlar:** ön ayarlardan ya da rastgele bir alev ekleyin, kaldırın, sırasını değiştirin ve her birinin karıştırmasını (add, screen, over) ve opaklığını ayarlayın. En çok 16 katman.
- **Görüntü boyutu:** pencerenin kendi boyutu, 21 ekrandan biri (4K’ya kadar bilgisayarlar, Android telefon ve tabletler, iPhone ve iPad’ler) ya da 16 ile 16384 piksel arasında herhangi bir genişlik ve yükseklik.
- **Derin yakınlaştırma:** sürükleme, tekerlek ve kıstırma hareketi görünümü kaydırır ve on milyar kata kadar büyütür. Görüntü hemen izler, görünüm durduğunda da net olarak yeniden çizilir.
- **Telefonlar:** kendine ait bir ekran. Yuvarlak bir düğme yeni bir görüntü üretir, bir kaydırma sonraki duvar kâğıdına geçer, küçük canlı görüntülerden oluşan bir sayfa stili seçtirir ve sakladığınız görüntüler bir koleksiyon oluşturur.
- **Dışa aktarma:** istediğiniz boyutta, üç kalite düzeyinde, arka planda işlenen bir PNG. Telefonda Fotoğraflar’a ya da galeriye kaydedilir veya paylaşılır.
- **Proje dosyaları:** `.frcq` dosyası sahneyi görünümü, tonu ve boyutuyla birlikte saklar; böylece kaydedildiği gibi açılır ([docs/scene-format.md](../scene-format.md)). Düz `.json` sahneleri ile flam3 ve Apophysis’in `.flame` dosyaları da açılır. Uygulama, açılışlar arasında bırakıldığı oturuma geri döner.
- **Güncellemeler:** bir pencere yayımlanan her sürümü notlarıyla listeler ve istediğinizi kurar. Windows’ta uygulama, kendi kendini değiştiren tek bir taşınabilir `.exe` dosyasıdır.
- **Tam yinelenebilirlik:** aynı sahne her makinede aynı pikselleri verir; bunu Linux, Windows ve macOS’taki testler doğrular ([docs/determinism.md](../determinism.md)).
- **Komut satırı aracı** (`fractique`): alevleri ve duvar kâğıtlarını PNG olarak işler, rastgele sahneler ve varyantlar üretir, `.flame` dosyalarını dönüştürür.
- **MCP sunucusu** (`fractique mcp`): bir yapay zekâ ajanı sahneler oluşturur, değiştirir ve işler ([docs/mcp.md](../mcp.md)).
- **HTTP sunucusu** (`fractique-server`): işleme kuyruğu ve sınırları olan bir HTTP API’si; `/mcp` adresinde yapay zekâ ajanları için MCP; web uygulamasını sunar, Docker imajı olarak gelir ([docs/server.md](../server.md)).

| | |
| --- | --- |
| ![emerald-star](../samples/emerald-star.png) | ![ember-vortex](../samples/ember-vortex.png) |
| `emerald-star` (iki katman) | `ember-vortex` |
| ![golden-bloom](../samples/golden-bloom.png) | ![solar-mandala](../samples/solar-mandala.png) |
| `golden-bloom` | `solar-mandala` |

Açık ve koyu duvar kâğıtları, her biri kendi düzeni ve malzemesiyle:

| | | | |
| --- | --- | --- | --- |
| ![waves](../samples/wallpaper-waves-light.png) | ![burst](../samples/wallpaper-burst-light.png) | ![tiles](../samples/wallpaper-tiles-light.png) | ![fan](../samples/wallpaper-fan-light.png) |
| `waves`, `glass` | `burst`, `glow` | `tiles`, `relief` | `fan`, `glass` |
| ![discs](../samples/wallpaper-discs-dark.png) | ![bars](../samples/wallpaper-bars-dark.png) | ![arcs](../samples/wallpaper-arcs-dark.png) | ![lines](../samples/wallpaper-lines-dark.png) |
| `discs`, `paper` | `bars`, `paper` | `arcs`, `carbon` | `lines`, `paper` |

## Hızlı başlangıç

Tek bir komut uygulamayı ve komut satırı aracını derler. Rust yoksa onu da kurar.

| Sistem | Komut |
| --- | --- |
| Linux, macOS | `./setup.sh` |
| Windows | `setup.cmd` |

Uygulamayı başlatın:

```sh
./target/release/fractique-gui
```

Ya da komut satırından işleyin:

```sh
# 4K görüntü olarak bir ön ayar
./target/release/fractique render --preset aurora-spiral --size 3840x2160 --quality high -o aurora.png

# telefon ekranı için 42 tohumunun rastgele alevi
./target/release/fractique render --random 42 --size 1080x2400 --quality mobile -o phone.png

# telefon için 7 tohumunun duvar kâğıdı, açık ve koyu
./target/release/fractique render --wallpaper 7 --size 1290x2796 -o wallpaper.png
./target/release/fractique render --wallpaper 7 --dark --size 1290x2796 -o wallpaper-dark.png

# rastgele bir sahneyi dosya olarak saklayın, değiştirin, işleyin
./target/release/fractique randomize 42 -o scene.json
./target/release/fractique render scene.json -o mine.png
```

Windows’ta programlar `target\release\fractique-gui.exe` ve `target\release\fractique.exe` konumundadır. Uygulamanın ve komutların yaptığı her şey: [docs/usage.md](../usage.md).

## Belgeler

Belgeler İngilizcedir.

| | |
| --- | --- |
| [docs/usage.md](../usage.md) | Uygulama, komutlar, kalite düzeyleri, bir sahnenin nasıl düzenleneceği |
| [docs/scene-format.md](../scene-format.md) | Sahne dosyasının her alanı, her varyasyon, levhalar, `.frcq` dosyası |
| [docs/mcp.md](../mcp.md) | MCP sunucusu: bir yapay zekâ ajanını bağlama, araçlar |
| [docs/server.md](../server.md) | HTTP sunucusu: uç noktalar, sınırlar, Docker |
| [docs/ios.md](../ios.md) | iPhone’da ya da iOS Simülatörü’nde derleme ve çalıştırma |
| [docs/releasing.md](../releasing.md) | Bir sürümün nasıl yayımlandığı, uygulamanın kendini nasıl güncellediği |
| [docs/architecture.md](../architecture.md) | Parçaların nasıl bir araya geldiği, bir işlemenin nasıl çalıştığı |
| [docs/determinism.md](../determinism.md) | Bir işlemenin neden tam olarak yinelendiği, neyin vaat edilip neyin edilmediği |
| [docs/performance.md](../performance.md) | Ölçülen işleme süreleri ve bellek |
| [docs/roadmap.md](../roadmap.md) | Neler yapıldı, sırada neler var |

## Lisans

[PolyForm Noncommercial License 1.0.0](../../LICENSE.md): ticari olmayan her kullanım için ücretsizdir. Ticari kullanım için yazardan ayrı bir lisans isteyin. Üçüncü taraf lisansları [THIRD_PARTY_NOTICES.md](../../THIRD_PARTY_NOTICES.md) dosyasında listelenmiştir. Katkılar: bkz. [CONTRIBUTING.md](../../CONTRIBUTING.md).

<div align="center">
  <img src="../../assets/icon/icon-256.png" alt="Fractique" width="128" height="128">
  <h1>Fractique</h1>
  <p><strong>같은 입력에서 어느 플랫폼에서나 같은 이미지를 만들어 내는 프랙탈 불꽃 및 배경화면 생성기.</strong></p>
  <p><sub>
    <a href="../../README.md">English</a> · <a href="README_UK.md">Українська</a> · <a href="README_RU.md">Русский</a> · <a href="README_DE.md">Deutsch</a> · <a href="README_FR.md">Français</a> · <a href="README_ES.md">Español</a> · <a href="README_IT.md">Italiano</a> · <a href="README_JA.md">日本語</a><br>
    <a href="README_CN.md">简体中文</a> · <a href="README_PT.md">Português</a> · <strong>한국어</strong> · <a href="README_HI.md">हिन्दी</a> · <a href="README_TR.md">Türkçe</a> · <a href="README_PL.md">Polski</a> · <a href="README_ID.md">Bahasa Indonesia</a>
  </sub></p>
</div>

Fractique는 두 가지 이미지를 만듭니다. 하나는 프랙탈 불꽃입니다. 가는 곡선과 부드러운 빛으로 이루어진, 빛나는 안개 같은 프랙탈입니다. 다른 하나는 배경화면입니다. 가장자리에 빛이 들고 부드러운 그림자가 지는 평평한 시트를 겹친 것으로, 각각 라이트 버전과 다크 버전이 있습니다. 프리셋을 고르거나, 무작위 이미지를 만들거나, 여러 불꽃을 겹친 다음 휴대폰 화면부터 4K 모니터까지 원하는 크기로 내보낼 수 있습니다. 장면은 작은 파일 하나이며, 같은 파일은 Windows, macOS, Linux, Android, iOS 어디서나 항상 정확히 같은 픽셀로 렌더링됩니다.

<p align="center">
  <img src="../samples/aurora-spiral.png" alt="aurora-spiral 프리셋: 어두운 배경 위의 청록색과 초록색 나선형 빛줄기" width="880">
</p>

## 기능

- **불꽃:** 49가지 배리에이션, 블렌드 모드가 있는 레이어, 회전 대칭과 거울 대칭, 밀도 추정, 글로우, 그리고 29개의 내장 프리셋.
- **배경화면:** 시드마다 하나씩, 14가지 레이아웃과 6가지 소재로 만들어지며, 각각 라이트 버전과 다크 버전이 있고 화면 크기에 맞게 배치됩니다.
- **앱:** Windows, macOS, Linux, Android, iOS, 웹을 하나의 코드베이스로 지원합니다. 이미지가 창을 가득 채우고 그 위에 유리 패널이 떠 있으며, 데스크톱에서는 앱 고유의 유리 제목 표시줄이 있습니다. 15개 언어 지원.
- **‘무작위’와 ‘변이’:** 모든 시드가 하나의 이미지입니다. 점이나 꽉 찬 사각형이 될 불꽃은 건너뜁니다. ‘변이’는 마음에 드는 이미지의 변형을 만듭니다.
- **레이어:** 프리셋이나 무작위 불꽃을 추가하고, 삭제하고, 순서를 바꾸고, 각 레이어의 블렌드(더하기, 스크린, 덮기)와 불투명도를 설정합니다. 최대 16개 레이어.
- **이미지 크기:** 창 자체의 크기, 59가지 화면(4K까지의 컴퓨터, Android 휴대폰과 태블릿, iPhone과 iPad) 중 하나, 또는 16~16384픽셀 사이의 임의의 너비와 높이.
- **딥 줌:** 드래그, 휠, 핀치로 보기를 옮기고 최대 100억 배까지 확대합니다. 이미지는 즉시 따라오고, 보기가 멈추면 선명하게 다시 그려집니다.
- **휴대폰:** 전용 화면. 둥근 버튼 하나로 새 이미지를 만들고, 스와이프로 다음 배경화면으로 넘어가고, 작은 라이브 이미지가 담긴 시트에서 스타일을 고르며, 보관한 이미지는 컬렉션이 됩니다.
- **내보내기:** 원하는 크기의 PNG를 화면에 보이는 그대로 즉시 저장하거나, 더 높은 세 가지 품질로 백그라운드에서 렌더링합니다. 휴대폰에서는 사진 앱이나 갤러리에 저장하거나 공유합니다. 모든 기능은 모든 플랫폼에서 무료입니다. App Store 버전(iPhone, iPad, Mac)에서만 2560 × 1440 픽셀 이상이나 사용자 지정 크기로 내보낼 때 한 번만 구입하면 되는 Fractique Pro가 필요합니다.
- **프로젝트 파일:** `.frcq` 파일은 장면을 보기, 톤, 크기와 함께 보관하므로 저장했던 그대로 열립니다([docs/scene-format.md](../scene-format.md)). 일반 `.json` 장면과 flam3 및 Apophysis의 `.flame` 파일도 열 수 있습니다. 앱은 다시 실행하면 마지막으로 떠났던 세션으로 돌아옵니다.
- **업데이트:** 창에서 출시된 모든 버전과 릴리스 노트를 보여 주고 그중 어느 것이든 설치합니다. Windows에서 앱은 스스로를 교체하는 포터블 `.exe` 하나입니다.
- **정확한 재현성:** 같은 장면은 어느 기기에서나 같은 픽셀을 만들며, Linux, Windows, macOS의 테스트로 확인합니다([docs/determinism.md](../determinism.md)).
- **명령줄 도구**(`fractique`): 불꽃과 배경화면을 PNG로 렌더링하고, 무작위 장면과 변형을 만들고, `.flame` 파일을 변환합니다.
- **MCP 서버**(`fractique mcp`): AI 에이전트가 장면을 만들고, 수정하고, 렌더링합니다([docs/mcp.md](../mcp.md)).
- **HTTP 서버**(`fractique-server`): 렌더링 대기열과 제한을 갖춘 HTTP API이며, `/mcp`에서 AI 에이전트용 MCP를 제공하고, 웹 앱을 제공하고 Docker 이미지로 배포됩니다([docs/server.md](../server.md)).

| | |
| --- | --- |
| ![emerald-star](../samples/emerald-star.png) | ![ember-vortex](../samples/ember-vortex.png) |
| `emerald-star` (레이어 2개) | `ember-vortex` |
| ![golden-bloom](../samples/golden-bloom.png) | ![solar-mandala](../samples/solar-mandala.png) |
| `golden-bloom` | `solar-mandala` |

라이트 배경화면과 다크 배경화면, 각각의 레이아웃과 소재:

| | | | |
| --- | --- | --- | --- |
| ![waves](../samples/wallpaper-waves-light.png) | ![burst](../samples/wallpaper-burst-light.png) | ![tiles](../samples/wallpaper-tiles-light.png) | ![fan](../samples/wallpaper-fan-light.png) |
| `waves`, `glass` | `burst`, `glow` | `tiles`, `relief` | `fan`, `glass` |
| ![discs](../samples/wallpaper-discs-dark.png) | ![bars](../samples/wallpaper-bars-dark.png) | ![arcs](../samples/wallpaper-arcs-dark.png) | ![lines](../samples/wallpaper-lines-dark.png) |
| `discs`, `paper` | `bars`, `paper` | `arcs`, `carbon` | `lines`, `paper` |

## 빠른 시작

명령 하나로 앱과 명령줄 도구를 빌드합니다. Rust가 없으면 설치합니다.

| 시스템 | 명령 |
| --- | --- |
| Linux, macOS | `./setup.sh` |
| Windows | `setup.cmd` |

앱 실행:

```sh
./target/release/fractique-gui
```

또는 명령줄에서 렌더링:

```sh
# 프리셋을 4K 이미지로
./target/release/fractique render --preset aurora-spiral --size 3840x2160 --quality high -o aurora.png

# 시드 42의 무작위 불꽃을 휴대폰 화면용으로
./target/release/fractique render --random 42 --size 1080x2400 --quality mobile -o phone.png

# 시드 7의 휴대폰용 배경화면, 라이트 버전과 다크 버전
./target/release/fractique render --wallpaper 7 --size 1290x2796 -o wallpaper.png
./target/release/fractique render --wallpaper 7 --dark --size 1290x2796 -o wallpaper-dark.png

# 무작위 장면을 파일로 보관하고, 수정하고, 렌더링
./target/release/fractique randomize 42 -o scene.json
./target/release/fractique render scene.json -o mine.png
```

Windows에서 프로그램은 `target\release\fractique-gui.exe`와 `target\release\fractique.exe`입니다. 앱과 명령으로 할 수 있는 모든 것: [docs/usage.md](../usage.md).

## 문서

문서는 영어로 되어 있습니다.

| | |
| --- | --- |
| [docs/usage.md](../usage.md) | 앱, 명령, 품질 수준, 장면 편집 방법 |
| [docs/scene-format.md](../scene-format.md) | 장면 파일의 모든 필드, 모든 배리에이션, 시트, `.frcq` 파일 |
| [docs/mcp.md](../mcp.md) | MCP 서버: AI 에이전트 연결, 도구 |
| [docs/server.md](../server.md) | HTTP 서버: 엔드포인트, 제한, Docker |
| [docs/ios.md](../ios.md) | iPhone 또는 iOS 시뮬레이터에서 빌드하고 실행하기 |
| [docs/releasing.md](../releasing.md) | 릴리스를 만드는 방법, 앱이 스스로 업데이트하는 방식 |
| [docs/architecture.md](../architecture.md) | 각 부분이 맞물리는 방식, 렌더링의 동작 원리 |
| [docs/determinism.md](../determinism.md) | 렌더링이 정확히 재현되는 이유, 보장하는 것과 보장하지 않는 것 |
| [docs/performance.md](../performance.md) | 측정한 렌더링 시간과 메모리 |
| [docs/roadmap.md](../roadmap.md) | 완료된 것과 앞으로의 계획 |

## 라이선스

[PolyForm Noncommercial License 1.0.0](../../LICENSE.md): 모든 비상업적 용도로 무료입니다. 상업적 용도는 작성자에게 별도의 라이선스를 문의하세요. 서드파티 라이선스는 [THIRD_PARTY_NOTICES.md](../../THIRD_PARTY_NOTICES.md)에 나와 있습니다. 기여: [CONTRIBUTING.md](../../CONTRIBUTING.md)를 참고하세요.

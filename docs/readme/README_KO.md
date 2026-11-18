<div align="center">
  <img src="../../assets/icon/icon-256.png" alt="Fractique" width="128" height="128">
  <h1>Fractique</h1>
  <p><strong>같은 입력이면 어떤 플랫폼에서도 같은 이미지를 만들어 내는 프랙탈 플레임 생성기.</strong></p>
  <p><sub>
    <a href="../../README.md">English</a> · <a href="README_UK.md">Українська</a> · <a href="README_RU.md">Русский</a> · <a href="README_DE.md">Deutsch</a> · <a href="README_FR.md">Français</a> · <a href="README_ES.md">Español</a> · <a href="README_IT.md">Italiano</a> · <a href="README_JA.md">日本語</a><br>
    <a href="README_CN.md">简体中文</a> · <a href="README_PT.md">Português</a> · <strong>한국어</strong> · <a href="README_HI.md">हिन्दी</a> · <a href="README_TR.md">Türkçe</a> · <a href="README_PL.md">Polski</a> · <a href="README_ID.md">Bahasa Indonesia</a>
  </sub></p>
</div>

Fractique는 프랙탈 플레임을 렌더링합니다. 가는 곡선과 부드러운 빛으로 이루어진, 빛나고 아련한 프랙탈입니다. 프리셋을 고르거나, 무작위 장면을 만들거나, 여러 플레임을 겹쳐서 휴대폰 화면부터 4K 배경화면까지 원하는 해상도로 렌더링할 수 있습니다. 장면은 작은 JSON 파일 하나이며, 같은 파일은 Windows, macOS, Linux, Android, iOS 어디에서나 항상 정확히 같은 픽셀로 렌더링됩니다.

<p align="center">
  <img src="../samples/aurora-spiral.png" alt="aurora-spiral 프리셋: 어두운 배경 위의 청록색과 초록색 나선형 빛줄기" width="880">
</p>

## 현재 동작하는 기능

- **렌더러**(`fractique-core`): 28가지 변형을 지원하는 프랙탈 플레임, 블렌드 모드가 있는 레이어, 회전 대칭과 거울 대칭, 밀도 추정, 글로우, 모든 해상도.
- **앱:** Windows, Linux, Android, iOS, 웹을 하나의 코드베이스로 지원하며, 플레임 위에 유리 패널이 떠 있고 15개 언어를 제공합니다.
- **프리셋:** 내장 장면 9개.
- **무작위 생성:** 모든 시드가 하나의 장면입니다. 점 하나나 꽉 찬 사각형이 될 장면은 건너뜁니다.
- **변이:** 마음에 드는 장면의 변형을 0에서 1 사이의 강도로 만듭니다.
- **정확한 재현성:** 같은 장면은 모든 기기에서 같은 픽셀이 되며, Linux, Windows, macOS의 테스트로 확인합니다([docs/determinism.md](../determinism.md)).
- **가져오기:** flam3와 Apophysis의 `.flame` 파일을 장면으로 변환합니다.
- **명령줄 도구**(`fractique`): 장면을 PNG로 렌더링합니다.
- **MCP 서버**(`fractique mcp`): AI 에이전트가 장면을 만들고, 바꾸고, 렌더링합니다([docs/mcp.md](../mcp.md)).
- **HTTP 서버**(`fractique-server`): 렌더링 대기열과 제한을 갖춘 HTTP API이며, 웹 앱을 제공하고 Docker 이미지로 배포됩니다([docs/server.md](../server.md)).

| | |
| --- | --- |
| ![emerald-star](../samples/emerald-star.png) | ![ember-vortex](../samples/ember-vortex.png) |
| `emerald-star` (레이어 2개) | `ember-vortex` |

## 설치

명령 하나로 전부 빌드합니다. Rust가 없으면 설치합니다.

| 시스템 | 명령 |
| --- | --- |
| Linux, macOS | `./setup.sh` |
| Windows | `setup.cmd` |

그다음:

```sh
# 프리셋을 4K 배경화면으로
./target/release/fractique render --preset aurora-spiral --size 3840x2160 --quality high -o aurora.png

# 시드 42의 무작위 장면을 휴대폰 화면용으로
./target/release/fractique render --random 42 --size 1080x2400 --quality mobile -o phone.png

# 무작위 장면을 파일로 저장하고, 수정한 뒤, 렌더링
./target/release/fractique randomize 42 -o scene.json
./target/release/fractique render scene.json -o mine.png
```

Windows에서는 도구가 `target\release\fractique.exe`에 있습니다. 모든 명령과 옵션: [docs/usage.md](../usage.md).

## 문서

문서는 영어로 되어 있습니다.

| | |
| --- | --- |
| [docs/usage.md](../usage.md) | 명령, 품질 수준, 장면 편집 방법 |
| [docs/scene-format.md](../scene-format.md) | 장면 파일의 모든 필드, 모든 변형 |
| [docs/mcp.md](../mcp.md) | MCP 서버: AI 에이전트 연결, 도구 |
| [docs/server.md](../server.md) | HTTP 서버: 엔드포인트, 제한, Docker |
| [docs/ios.md](../ios.md) | iPhone 또는 iOS 시뮬레이터에서 빌드하고 실행하기 |
| [docs/architecture.md](../architecture.md) | 각 부분이 어떻게 맞물리는지, 렌더링이 어떻게 동작하는지 |
| [docs/determinism.md](../determinism.md) | 렌더링이 정확히 재현되는 이유, 보장되는 것과 보장되지 않는 것 |
| [docs/performance.md](../performance.md) | 측정한 렌더링 시간과 메모리 |
| [docs/roadmap.md](../roadmap.md) | 완료된 것과 다음에 할 것 |

## 라이선스

[PolyForm Noncommercial License 1.0.0](../../LICENSE.md): 모든 비상업적 용도에는 무료입니다. 상업적 용도는 작성자에게 별도 라이선스를 문의하세요. 서드파티 라이선스는 [THIRD_PARTY_NOTICES.md](../../THIRD_PARTY_NOTICES.md)에 정리되어 있습니다. 기여: [CONTRIBUTING.md](../../CONTRIBUTING.md) 참고.

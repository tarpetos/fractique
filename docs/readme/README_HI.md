<div align="center">
  <img src="../../assets/icon/icon-256.png" alt="Fractique" width="128" height="128">
  <h1>Fractique</h1>
  <p><strong>एक फ्रैक्टल फ्लेम जनरेटर जो एक ही इनपुट से हर प्लेटफ़ॉर्म पर एक ही तस्वीर बनाता है।</strong></p>
  <p><sub>
    <a href="../../README.md">English</a> · <a href="README_UK.md">Українська</a> · <a href="README_RU.md">Русский</a> · <a href="README_DE.md">Deutsch</a> · <a href="README_FR.md">Français</a> · <a href="README_ES.md">Español</a> · <a href="README_IT.md">Italiano</a> · <a href="README_JA.md">日本語</a><br>
    <a href="README_CN.md">简体中文</a> · <a href="README_PT.md">Português</a> · <a href="README_KO.md">한국어</a> · <strong>हिन्दी</strong> · <a href="README_TR.md">Türkçe</a> · <a href="README_PL.md">Polski</a> · <a href="README_ID.md">Bahasa Indonesia</a>
  </sub></p>
</div>

Fractique फ्रैक्टल फ्लेम बनाता है: पतली रेखाओं और नरम रोशनी से बने चमकते, धुएँ जैसे फ्रैक्टल। आप कोई प्रीसेट चुनते हैं, यादृच्छिक दृश्य बनाते हैं या कई फ्लेम एक के ऊपर एक रखते हैं, और उसे किसी भी रिज़ॉल्यूशन में रेंडर करते हैं, फ़ोन की स्क्रीन से लेकर 4K वॉलपेपर तक। एक दृश्य एक छोटी JSON फ़ाइल है, और वही फ़ाइल Windows, macOS, Linux, Android और iOS पर हमेशा बिल्कुल वही पिक्सेल देती है।

<p align="center">
  <img src="../samples/aurora-spiral.png" alt="aurora-spiral प्रीसेट: गहरे पृष्ठभूमि पर फ़िरोज़ी और हरी सर्पिल लकीरें" width="880">
</p>

## अभी क्या काम करता है

- **रेंडरर** (`fractique-core`): 49 वेरिएशन वाले फ्रैक्टल फ्लेम, ब्लेंड मोड वाली लेयर, घूर्णन और दर्पण सममिति, घनत्व अनुमान, आभा, कोई भी रिज़ॉल्यूशन।
- **ऐप:** Windows, Linux, Android, iOS और वेब के लिए एक ही कोडबेस, फ्लेम के ऊपर काँच के पैनल, 15 भाषाओं में।
- **प्रीसेट:** 9 बने-बनाए दृश्य।
- **यादृच्छिक जनरेटर:** हर सीड एक दृश्य है; जो दृश्य केवल एक बिंदु या भरा हुआ वर्ग बनते, उन्हें छोड़ दिया जाता है।
- **उत्परिवर्तन:** आपके पसंदीदा दृश्य का एक रूपांतर, 0 से 1 तक की तीव्रता के साथ।
- **सटीक दोहराव:** वही दृश्य हर मशीन पर वही पिक्सेल देता है; इसे Linux, Windows और macOS पर टेस्ट जाँचते हैं ([docs/determinism.md](../determinism.md))।
- **आयात:** flam3 और Apophysis की `.flame` फ़ाइलें दृश्यों में बदल जाती हैं।
- **कमांड लाइन टूल** (`fractique`): दृश्यों को PNG में रेंडर करता है।
- **MCP सर्वर** (`fractique mcp`): एक AI एजेंट दृश्य बनाता, बदलता और रेंडर करता है ([docs/mcp.md](../mcp.md))।

| | |
| --- | --- |
| ![emerald-star](../samples/emerald-star.png) | ![ember-vortex](../samples/ember-vortex.png) |
| `emerald-star` (दो लेयर) | `ember-vortex` |

## सेटअप

एक कमांड सब कुछ बिल्ड कर देता है। Rust न हो तो वह उसे इंस्टॉल कर देता है।

| सिस्टम | कमांड |
| --- | --- |
| Linux, macOS | `./setup.sh` |
| Windows | `setup.cmd` |

फिर:

```sh
# एक प्रीसेट, 4K वॉलपेपर के रूप में
./target/release/fractique render --preset aurora-spiral --size 3840x2160 --quality high -o aurora.png

# सीड 42 का यादृच्छिक दृश्य, फ़ोन की स्क्रीन के लिए
./target/release/fractique render --random 42 --size 1080x2400 --quality mobile -o phone.png

# यादृच्छिक दृश्य को फ़ाइल में रखें, बदलें, रेंडर करें
./target/release/fractique randomize 42 -o scene.json
./target/release/fractique render scene.json -o mine.png
```

Windows पर टूल `target\release\fractique.exe` में है। सभी कमांड और विकल्प: [docs/usage.md](../usage.md)।

## दस्तावेज़

दस्तावेज़ अंग्रेज़ी में हैं।

| | |
| --- | --- |
| [docs/usage.md](../usage.md) | कमांड, गुणवत्ता स्तर, दृश्य को कैसे संपादित करें |
| [docs/scene-format.md](../scene-format.md) | दृश्य फ़ाइल का हर फ़ील्ड, हर वेरिएशन |
| [docs/mcp.md](../mcp.md) | MCP सर्वर: AI एजेंट जोड़ना, टूल |
| [docs/ios.md](../ios.md) | iPhone या iOS सिम्युलेटर पर बिल्ड करना और चलाना |
| [docs/architecture.md](../architecture.md) | हिस्से आपस में कैसे जुड़ते हैं, रेंडर कैसे काम करता है |
| [docs/determinism.md](../determinism.md) | रेंडर बिल्कुल वैसा ही क्यों दोहराता है, किस बात का वादा है और किसका नहीं |
| [docs/performance.md](../performance.md) | मापा गया रेंडर समय और मेमोरी |
| [docs/roadmap.md](../roadmap.md) | क्या हो चुका है और आगे क्या है |

## लाइसेंस

[PolyForm Noncommercial License 1.0.0](../../LICENSE.md): किसी भी गैर-व्यावसायिक उपयोग के लिए निःशुल्क। व्यावसायिक उपयोग के लिए लेखक से अलग लाइसेंस माँगें। तृतीय-पक्ष लाइसेंस [THIRD_PARTY_NOTICES.md](../../THIRD_PARTY_NOTICES.md) में सूचीबद्ध हैं। योगदान: [CONTRIBUTING.md](../../CONTRIBUTING.md) देखें।

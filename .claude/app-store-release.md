# App Store release profile: Fractique

Facts and the owner's standing choices for a release to the App Store, read by a coding agent's
`app-store-release` skill. The order is in [docs/releasing.md](../docs/releasing.md#the-stores): the version is
tagged on GitHub first, and a store gets a build of that tag. No passwords, keys or tokens here.

## The app

| | |
| --- | --- |
| App Store Connect app id | 6836122841 |
| Bundle identifier | `app.fractique` (iOS and macOS, one app record; the framework is `app.fractique.ProWatch`) |
| Team id | `73L32W9C88` |
| Store platforms | iOS (iPhone and iPad, one build) and macOS |
| In-app purchase | `app.fractique.pro`, Non-Consumable, one product for both platforms |
| Listing | English (U.S.); the app itself has 15 languages |
| Countries | 172, chosen by the owner; the same for the app and for the purchase |
| Account holder | The owner: signs in to Xcode and App Store Connect, answers 2FA, accepts agreements |
| Repository | `tarpetos/fractique`, default branch `master`, pull requests squash-merged |

## GitHub release

[docs/releasing.md](../docs/releasing.md) describes it. In short:

- The version is in four files (`Cargo.toml`, `crates/fractique-app/Packager.toml`, `ios/project.yml`,
  `mac/project.yml`) and a `## <version>` section in `CHANGELOG.md`, which becomes the release notes.
- `git tag v<version> && git push origin v<version>` starts the `Release` workflow: it builds the installers, the
  APK and the Docker image and publishes the release. Watch it with `gh run watch`; it is finished when
  `gh release view v<version>` shows the release.
- The tag's run builds every platform, iOS and the Mac App Store included, and sends nothing out unless all of
  them succeed: then it uploads the two Apple builds, pushes the Docker image and publishes the release.
- Started by hand, the `Release` workflow builds and checks without uploading or publishing.

## iOS (iPhone and iPad)

| | |
| --- | --- |
| Test before the tag | `FRACTIQUE_FEATURES=store ./scripts/ios/run-iphone.sh` on a device, `./scripts/ios/run-simulator.sh` in the Simulator |
| Build and upload | The tag's `Release` run. From a Mac only to repeat it: `export PATH=/opt/homebrew/opt/rustup/bin:$PATH`, then `./scripts/ios/testflight.sh` in a clean checkout of the tag (it refuses elsewhere) |
| Upload an archive again | In `ios/`: `xcodebuild -exportArchive -archivePath build/Fractique.xcarchive -exportOptionsPlist ExportOptions.plist -exportPath build/export -allowProvisioningUpdates DEVELOPMENT_TEAM=73L32W9C88` |
| Build number | UTC date and time, `yymmddHHMM`, made by the script |
| Processed build | 5 to 30 minutes after the upload; the Internal TestFlight group gets it by itself |
| Check list | "Testing it" in [docs/ios.md](../docs/ios.md#testing-it) |
| Version page | `https://appstoreconnect.apple.com/apps/6836122841/distribution/ios/version/inflight` |
| Screenshots | iPhone slot "medium display": 1206 × 2622 (1320 × 2868 and 1290 × 2796 are refused). iPad 13": 2064 × 2752. Six each |

## macOS

| | |
| --- | --- |
| Test before the tag | `./scripts/mac/appstore.sh --run`: the store build in its sandbox on this Mac |
| Before the upload | `nm -u target/mac-app-store/fractique-gui \| grep CGS` shows `_CGShieldingWindowLevel` alone |
| Build and upload | The tag's `Release` run. From a Mac only to repeat it: `./scripts/mac/appstore.sh` in a clean checkout of the tag |
| Screenshots | 2880 × 1800 |

## Both

- The scripts stop on an error, but a pipe after them (`| tail`) hides it: run them unpiped and look for
  `** EXPORT SUCCEEDED **`.
- Each script starts with `rm -rf build`: a second run removes the archive of the first.
- "missing Xcode-Token … exportArchive Failed to Use Accounts": Xcode is signed out. The owner signs in under
  Xcode, Settings, Accounts.
- All submissions and the reviewer's messages: `https://appstoreconnect.apple.com/apps/6836122841/distribution/reviewsubmissions`.
- A build attaches only to a version of its own number.
- Store screenshots are made in the Simulator from scenes of `docs/gallery/`: `fractique convert <file>.frcq
  --document` gives the JSON to put under `"session"` in the app's `Documents/settings.ron` while the app is
  closed. Status bar: `xcrun simctl status_bar <udid> override --time "9:41"`.

## Standing choices of the owner

- The release option is **manual**, phased release off. A version is released when the owner says so.
- Merging a pull request, pushing a tag, submitting and releasing: each on the owner's word.
- Everything is prepared first, the owner tries the build, then it is submitted.
- Screenshots are shown in the chat before they are uploaded. Pictures from the gallery, a single object on a dark
  background rather than a full-screen texture; never the default presets.
- Commits: check `git config user.email` first; no attribution lines in commits or pull requests.

## First submission

What passed the first review:

- **The recording:** made by the owner on an iPhone with a sandbox account, from the TestFlight build that was
  submitted. Launch from the Home Screen, the three kinds of pictures, colours, zoom, the controls, the three free
  saves and the pictures in Photos, the Fractique Pro sheet, the sandbox purchase, the unlocked features. About six
  minutes, compressed from 590 MB to 39 MB, attached by the owner under App Review Information.
- **The notes,** the six numbered answers in one field:

```
1. Screen recording. Attached: a recording made on an iPhone, starting from launching the app. It shows the three kinds of pictures (Flames, Fractals, Shapes), colours, zoom, the controls, the free limit of 3 saves a day with the exported pictures in Photos, the Fractique Pro purchase in the sandbox, and the unlocked features afterwards (custom export size, no daily limit, hiding the panels).

2. Purpose and audience. Fractique is a generator of abstract wallpapers for iPhone, iPad and Mac. It makes three kinds of pictures entirely on the device: fractal flames, classic fractals (Mandelbrot, Julia, Burning Ship, Newton, strange attractors, the Mandelbulb and others) and calm geometric wallpapers ("Shapes"). It is for anyone who wants a distinctive, original wallpaper without searching image sites: every tap makes a new picture, sized exactly for the screen. The value is unlimited, unique, high-resolution wallpapers generated mathematically, with no internet needed.

3. How to use it. No account, login, setup or sample files are needed. Open the app: a picture is already on screen. The three tabs at the top switch between Flames, Fractals and Shapes; the dice button makes a new picture; the colour row changes the palette; pinch to zoom; press and hold to preview the picture as a lock screen. The download button (bottom right) opens Export, where you choose a size and quality and tap Save to Photos or Share.
In-app purchase: without Fractique Pro the app saves or shares 3 pictures a day, at any listed size. Fractique Pro (non-consumable, one-time) removes the daily limit, unlocks typing a custom export size, and lets you hide the panels to see the picture alone (the eye button). To see the purchase sheet: open Export, type a size under Custom and tap Apply; or save three pictures and tap Save to Photos once more; or tap the eye button. Restore purchases is on the same sheet. The purchase is shared by the iPhone, iPad and Mac versions.

4. External services. None for the core functionality: every picture is computed on the device. The only external service is Apple's StoreKit, for the Fractique Pro purchase. No analytics, advertising, AI, account or other third-party services.

5. Regional differences. None. The app works the same in every region. The interface is available in 15 languages.

6. Regulated industry or third-party material. Not applicable. Fractique is not in a regulated industry, and all pictures are generated by the app's own code. It contains no third-party images or other protected material.
```

- App Privacy: "Data Not Collected", the policy at `docs/privacy.md` on GitHub. Age rating 4+. No third-party
  content. Family Sharing of the purchase is off.

## Lessons

- A new app or a new platform sends the recording and the six answers with its first submission: without them
  the answer is "Guideline 2.1 - Information Needed - New App Submission".
- A dependency can link a private API, and an automated check then refuses the Mac build
  (`_CGSSetWindowBackgroundBlurRadius`, from `winit`): the `nm` check above runs before every Mac upload.
- A fix that is wanted after a tag is the next version; a tag is never built with other code.

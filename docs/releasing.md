# Releasing and updates

A release is a version tag. Pushing `v0.2.0` makes GitHub build every installer, sign them and publish them with
that version's part of the changelog; installed copies of the app then find it by themselves.

## What a release contains

| File | For | Updates itself |
| --- | --- | --- |
| `Fractique_<version>_x64.exe` | Windows: the app itself, one file, no installer | yes |
| `fractique-cli_<version>_x64.exe` | Windows: the command line tool | no |
| `Fractique_<version>_x86_64.AppImage` | Linux, any distribution | yes |
| `Fractique_<version>_amd64.deb` | Debian, Ubuntu | no: it points at the download page |
| `Fractique_<version>_aarch64.dmg` | macOS, Apple Silicon | yes (the app inside) |
| `Fractique_<version>_aarch64.app.tar.gz` | what the macOS update downloads | |
| `Fractique_<version>_android.apk` | Android | no: Android installs an APK only by hand |
| `latest.json` | the update manifest the app reads | |
| `SHA256SUMS.txt` | checksums | |

The Linux and macOS packages carry the app (`fractique-gui`) and the command line tool (`fractique`).

## How to release

1. Set the new version in two places: `version` under `[workspace.package]` in `Cargo.toml` and `version` in
   `crates/fractique-app/Packager.toml`. The build scripts refuse to build when they differ.
2. Add a section `## <version>` at the top of `CHANGELOG.md`. It becomes the release's notes and what the app
   shows under "Updates and what's new".
3. Commit, then tag and push the tag:

   ```sh
   git tag v0.2.0
   git push origin v0.2.0
   ```

The `Release` workflow does the rest. It refuses a tag that does not match the version in `Cargo.toml`. Started by
hand without a tag, it builds the installers and keeps them as artifacts without publishing.

To build installers on your own machine: `scripts/build/build-linux.sh`, `build-mac.sh`, `build-win.ps1`. They
write to `dist/`.

| Target | Built by | With |
| --- | --- | --- |
| Windows | `scripts/build/build-win.ps1` | The two programs copied as they are; no installer |
| Linux | `scripts/build/build-linux.sh [--formats appimage,deb]` | `cargo-packager` |
| macOS | `scripts/build/build-mac.sh [--formats app,dmg]` | `cargo-packager`. The app is not signed by Apple: copied from the `.dmg` it needs `xattr -dr com.apple.quarantine /Applications/Fractique.app` |
| Android | `cargo apk build --release --lib` in `crates/fractique-app`, as the workflow does | `cargo-apk`, the Android NDK, Java 17, the keystore |
| iOS | `scripts/ios/` ([ios.md](ios.md)); not part of a release | Xcode |
| Web | `crates/fractique-app/index.html` is a [Trunk](https://trunkrs.dev) page; not part of a release | Trunk |

## How the app updates

```mermaid
flowchart LR
    APP["The app, at start"] -- "releases/latest/download/latest.json" --> GH[("GitHub Releases")]
    GH -- "version, notes, installer URL, signature" --> APP
    APP -- "newer?" --> ROW["Title bar: the version turns into 'vX ↑'"]
    ROW --> WIN["Updates window:<br/>every release and its notes"]
    WIN -- "Update to X / Install X" --> DL["Download, check the signature"]
    DL --> INST["Install on close, or Restart now"]
```

- **The check** reads `latest.json` of the newest release. It happens once at start, in the background, and on
  **Check for updates**. When a newer version is out, the version in the title bar shows it with an arrow; a click
  on it opens the updates window.
- **The list** in the updates window comes from the GitHub Releases API: every full release from `0.1.0` on, with
  its date and notes. Pre-releases and drafts are left out.
- **Any version can be installed**, older ones too: each release has its own `latest.json`, and the app reads the
  one of the chosen version.
- **The download is verified** against the signature in the manifest with the public key built into the app
  (`PUBKEY` in `crates/fractique-app/src/update.rs`). A file signed with another key is refused.
- **Installing** happens when the app closes, or at once with **Restart now**. On Windows the app is one portable
  program file: it renames its own file to `.old`, puts the new one in its place and removes the old one at the
  next start. The settings are not in that file: they stay in the data folder
  ([usage.md](usage.md#what-the-app-keeps)).

Which copy can do what:

| Copy | Behaviour |
| --- | --- |
| The Windows program in a folder it can write to, the AppImage, the macOS app | Updates itself |
| The Windows program in a folder it cannot write to (`Program Files`) | Shows the versions and opens the download page |
| The `.deb`, or a binary copied by hand | Shows the versions and opens the download page |
| Built from a checkout (`cargo build`, `setup.sh`) | Shows the versions, installs nothing |
| Android, iOS, the web | No updates window: the stores, TestFlight, or a new APK by hand, replace the app; the web is always the newest |

The release scripts mark a build as a release by setting `FRACTIQUE_RELEASE` while compiling. Without it the app
counts as built from a checkout.

The repository must be public for this to work: the app asks GitHub without signing in.

## The keys

Two keys sign a release. Neither is in the repository; the workflow reads them from the repository's secrets.

| Secret | What it signs | If it is lost |
| --- | --- | --- |
| `CARGO_PACKAGER_SIGN_PRIVATE_KEY`, `CARGO_PACKAGER_SIGN_PRIVATE_KEY_PASSWORD` | The desktop installers, for the updater | Installed copies refuse every later update: people must install by hand once |
| `ANDROID_KEYSTORE_BASE64`, `ANDROID_KEYSTORE_PASSWORD` | The APK | Android refuses to install a later APK over the app: it must be removed first |

Keep copies of both outside GitHub: a secret cannot be read back from there.

A new updater key is made with `cargo packager signer generate`; its public half replaces `PUBKEY`.

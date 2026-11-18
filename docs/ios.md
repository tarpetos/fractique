# iOS

Fractique for iPhone and iPad is the same Rust code as on every other platform. It is built on a Mac, because
Apple's tools exist only there. Two scripts do the work; both build the app first.

| Script | Where the app runs | Needs |
| --- | --- | --- |
| `scripts/ios/run-simulator.sh` | The iOS Simulator on the Mac | Xcode |
| `scripts/ios/run-iphone.sh` | The iPhone connected by cable | Xcode 15 or newer, signing (below) |

## Once, on the Mac

1. Install **Xcode** from the App Store and open it once, so it installs the iOS platform.
2. Get the code: `git clone git@github.com:tarpetos/fractique.git` (the repository is private: its owner adds you
   as a collaborator first), then `cd fractique`.

Rust is installed by the scripts when it is missing.

## In the simulator

```sh
./scripts/ios/run-simulator.sh
```

The first build takes a few minutes. The Simulator opens with Fractique in it. No Apple account is needed.

What to look at: the picture fills the screen; the **Flames** and **Shapes** switch at the top and the round
buttons at the bottom right stay clear of the notch and the home bar; the round button makes a new picture, a swipe
goes to the next wallpaper, **Style** opens the sheet of small pictures and **Fine-tune** in it the sliders.
[usage.md](usage.md#on-a-phone) describes the screen.

The same run can be started on GitHub, without a Mac: the `iOS simulator` workflow, started by hand, builds the
app, starts it in a simulator and keeps two screenshots and the logs as an artifact.

## On an iPhone

### Signing

Apple lets an app onto an iPhone only when it is signed. The script finds what it needs on the Mac by itself:

- an **Apple Development** certificate in the keychain;
- a **provisioning profile** that covers the app `app.fractique` (or any app, a wildcard profile) and this iPhone.

The simplest way to have both, when they are not there yet:

1. In Xcode: **File, New, Project, iOS, App**. Product name `Fractique`, organization identifier `app`, so that the
   bundle identifier reads `app.fractique`. Choose your team.
2. Connect the iPhone, pick it as the run destination and press **Run** once. Xcode creates the certificate and the
   profile and registers the iPhone. The empty app it installs can be deleted.

If Apple refuses the identifier `app.fractique` (someone else's team may hold it), use your own, for example
`com.yourname.fractique`, in Xcode and tell the script:

```sh
FRACTIQUE_BUNDLE_ID=com.yourname.fractique ./scripts/ios/run-iphone.sh
```

### The iPhone

- Connect it by cable, unlock it and tap **Trust**.
- Turn on **Settings, Privacy & Security, Developer Mode** (iOS 16 and newer; the iPhone restarts).

### Run

```sh
./scripts/ios/run-iphone.sh
```

It builds, signs, installs and starts the app, and then shows what the app prints. With a free Apple account the
app stops opening after 7 days; with a paid one, after a year. Running the script again renews it.

### For an iPhone that is somewhere else

```sh
./scripts/ios/run-iphone.sh --ipa            # signed: installs on the iPhones in the provisioning profile
./scripts/ios/run-iphone.sh --unsigned-ipa   # not signed: the one who installs it signs it
```

Both write `target/ios/Fractique.ipa`. The signed file installs on any iPhone registered in the team's profile,
from a Mac (Apple Configurator, Xcode's Devices window) or from Windows with a sideloading tool. The unsigned file
is for a tool such as Sideloadly, which signs it with the Apple account of the person who installs it; with a free
account the app works for 7 days.

| Setting | Meaning |
| --- | --- |
| `FRACTIQUE_BUNDLE_ID` | The app's identifier, when it is not `app.fractique` |
| `FRACTIQUE_SIGN_IDENTITY` | The certificate to sign with, when the keychain holds several |
| `FRACTIQUE_DEVICE` | The iPhone's identifier from `xcrun devicectl list devices`, when several are connected |

## What to report back

- Does it start? If not, the text the script printed.
- A screenshot of the app with the styles open and one with them closed.
- How long the full picture takes (the line under the picture's name shows it) and whether the sliders in
  **Fine-tune** feel smooth.
- Whether **Export** puts the picture into Photos, and whether **Share** opens the share sheet. The first save
  asks for permission to add to the photo library.

## TestFlight

```sh
./scripts/ios/testflight.sh
```

It builds Fractique, signs it for the App Store and uploads it; every run gets a new build number. TestFlight offers
the build to the testers after Apple processes it (5 to 30 minutes).

Once per Mac: `brew install xcodegen`, and Xcode signed in (Settings, Accounts) to the Apple account of the team that
owns the app `app.fractique` in App Store Connect. `FRACTIQUE_TEAM_ID` picks another team.

`ios/` holds the Xcode project that wraps the Rust binary: Xcode compiles an empty `stub.c`, then
`ios/build-rust.sh` puts the Rust binary in its place before Xcode signs the app.

## What the app needs from iOS

| | |
| --- | --- |
| iOS version | 15.0 or newer |
| Devices | iPhone and iPad, upright and on either side |
| Photos | Permission to add pictures (`NSPhotoLibraryAddUsageDescription`), asked at the first **Save to Photos** |
| Files | The app's `Documents` folder: the settings (`settings.ron`) and the exported pictures (`exports`) |

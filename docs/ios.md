# iOS

Fractique for iPhone and iPad is the same Rust code as on every other platform. It is built on a Mac, because
Apple's tools exist only there. Two scripts do the work; both build the app first.

| Script | Where the app runs | Needs |
| --- | --- | --- |
| `scripts/ios/run-simulator.sh` | The iOS Simulator on the Mac | Xcode |
| `scripts/ios/run-iphone.sh` | The iPhone connected by cable | Xcode 15 or newer, signing (below) |

> The scripts were written without a Mac at hand and have not run yet. If one stops, send what it printed.

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

What to look at: the flame fills the screen, the toolbar at the bottom and the status capsule at the top stay
clear of the notch and the home bar, **Controls** opens the inspector, the sliders and **Randomize** work.

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

| Setting | Meaning |
| --- | --- |
| `FRACTIQUE_BUNDLE_ID` | The app's identifier, when it is not `app.fractique` |
| `FRACTIQUE_SIGN_IDENTITY` | The certificate to sign with, when the keychain holds several |
| `FRACTIQUE_DEVICE` | The iPhone's identifier from `xcrun devicectl list devices`, when several are connected |

## What to report back

- Does it start? If not, the text the script printed.
- A screenshot of the app with the inspector open and one with it closed.
- How long the full picture takes (the status capsule shows it) and whether the sliders feel smooth.

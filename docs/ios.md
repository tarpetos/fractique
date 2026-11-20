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
buttons at the bottom right stay clear of the notch and the home bar; the large one makes a new picture, a swipe
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

### From GitHub

The **Release** workflow does the same on one of GitHub's Macs, beside the installers of the other systems: for
every version tag, and when it is started by hand (Actions, Release, Run workflow; **ios** there builds this
alone). The build goes to TestFlight only, so to the testers of the team; a refusal by Apple does not hold the
installers back. It has no signed-in Xcode, so it signs with an App Store Connect API key, which the team's
account holder or an admin makes once:

1. In App Store Connect: **Users and Access, Integrations, App Store Connect API, Team Keys**, the plus. Name it,
   give it the role **Admin** (a lower role cannot fetch the certificate that signs for the App Store), generate.
2. Download the `.p8` file. Apple offers it once. Note the **Key ID** beside the key and the **Issuer ID** above the
   list.
3. Put the three into the repository's secrets, from a terminal in the repository:

```sh
gh secret set ASC_KEY_P8 < ~/Downloads/AuthKey_XXXXXXXXXX.p8
gh secret set ASC_KEY_ID
gh secret set ASC_ISSUER_ID
```

The last two ask for the value. Keep the `.p8` file somewhere safe outside GitHub: a secret cannot be read back.
Whoever has the key can upload builds and manage the team's apps; **Revoke** beside the key in App Store Connect
ends that.

The same key works on a Mac without a signed-in Xcode: `ASC_KEY_PATH` (the file), `ASC_KEY_ID` and `ASC_ISSUER_ID`
in front of `scripts/ios/testflight.sh`.

`ios/` holds the Xcode project that wraps the Rust binary: Xcode compiles an empty `stub.c`, then
`ios/build-rust.sh` puts the Rust binary in its place before Xcode signs the app.

## Fractique Pro, the purchase

The build that goes to the App Store (`ios/build-rust.sh`, so `scripts/ios/testflight.sh`) is compiled with the
`store` feature: an export of 3 686 400 pixels (2560 × 1440) or more and a size typed in need the purchase
**Fractique Pro** ([usage.md](usage.md#free-and-fractique-pro)). The scripts for the Simulator and for a
connected iPhone build without it, so there everything is free; `FRACTIQUE_FEATURES=store` in front of them
builds the App Store's variant.

The app asks the App Store for one product and sells nothing else:

| | |
| --- | --- |
| Product ID | `app.fractique.pro` |
| Type | Non-Consumable |
| Reference name | Fractique Pro |
| Price | 4.99 USD (the tier the owner chose); the app shows whatever the App Store returns, in the buyer's currency |

### Once, in App Store Connect (the account holder)

1. **Business, Agreements:** accept the **Paid Apps** agreement and fill in the bank account and the tax forms.
   Until it is active the App Store returns no product, and the sheet says that the App Store did not answer.
2. **Apps, Fractique, Monetization, In-App Purchases, +:** type **Non-Consumable**, reference name
   `Fractique Pro`, product ID `app.fractique.pro` exactly. A product ID cannot be changed or used again later.
3. In the product: **Availability** (all countries), **Price** (4.99 USD), and under **App Store Localization** a
   display name and a description for each language of the listing, for example "Fractique Pro" and "Export
   pictures at 2560 × 1440 and larger, and at any custom size."
4. **Review Information:** a screenshot of the Fractique Pro sheet and a note where to find it ("Export, choose
   3840 × 2160, Save"). The state must read **Ready to Submit**.
5. The first purchase of an app is reviewed together with a version of the app: on the version's page, under
   **In-App Purchases and Subscriptions**, add Fractique Pro before **Add for Review**.
6. **Users and Access, Sandbox, Test Accounts:** create a sandbox tester (an email address that is not an Apple
   Account yet).

Nothing is set in Xcode: an iOS app may sell purchases without an entitlement, and the binary links StoreKit by
itself.

### Testing it

On a device, with a build from `scripts/ios/testflight.sh` installed through TestFlight, or with
`FRACTIQUE_FEATURES=store ./scripts/ios/run-iphone.sh`. A TestFlight build buys in the sandbox by itself and
charges nothing; for a build installed by cable, sign in with the sandbox tester under **Settings, Developer,
Sandbox Apple Account**.

| Step | Expected |
| --- | --- |
| Open **Export** | 2560 × 1440, 3840 × 2160 and the other large sizes have a lock, 1920 × 1080 and 1290 × 2796 have none; **Custom** has a lock; no Android size is listed |
| Choose 1920 × 1080, **Save to Photos** | The picture is saved, no sheet |
| Choose 3840 × 2160, **Save to Photos** | The Fractique Pro sheet with the price in the tester's currency |
| The round button with a cross, top right | Back in the export sheet, nothing saved |
| **Save** again, **Buy**, confirm | The system's purchase dialog; then the sheet closes, the export starts by itself and the picture is in Photos; the locks are gone |
| Close the app, turn on Airplane Mode, open it | Still no locks |
| Delete the app, install it again, open **Export**, press a locked size, **Restore purchases** | The locks go without paying again |
| The same with a sandbox tester who never bought | "No earlier purchase was found for this Apple Account." |
| Start a purchase and cancel the system's dialog | The sheet is back at **Buy**, no message |
| Airplane Mode on a device that never bought, open the sheet | The App Store's own words or "The App Store did not answer", and **Try again** |
| **Fine-tune, Size**: press a locked size or **Apply** under **Custom** | The sheet; after a purchase the size is set |
| **Share** with a locked size on an iPad | The sheet, and after the purchase the share popover |
| Ask to Buy (a child's sandbox account), if available | "The purchase is waiting for approval."; the locks go when it is approved |

A sandbox purchase is cleared under **Settings, Developer, Sandbox Apple Account, Manage, Reset Eligibility** or
by a new sandbox tester. The purchase is kept on the device in the app's preferences (`NSUserDefaults`, the key
`app.fractique.pro`); deleting the app removes it, **Restore purchases** brings it back. A refund does not lock
the app again.

### A Mac App Store build

No script builds one yet: `scripts/build/build-mac.sh` makes the app that is downloaded from the releases, which
has no purchase. A build for the Mac App Store has to:

- compile with `cargo build --release -p fractique-app --features store` and without `FRACTIQUE_RELEASE`, so
  that the app does not install updates by itself;
- have its own app record in App Store Connect (or the iOS one made universal), where `app.fractique.pro` is
  offered too;
- be signed with an Apple Distribution certificate, the App Sandbox entitlement
  (`com.apple.security.app-sandbox`, and `com.apple.security.files.user-selected.read-write` for Open, Save and
  Export, `com.apple.security.network.client` for the App Store) and a provisioning profile, and be packed as a
  `.pkg`.

The purchase code is the same as on iOS (StoreKit's `SKPaymentQueue`), and no step of it has been run on a Mac.

## What the app needs from iOS

| | |
| --- | --- |
| iOS version | 15.0 or newer |
| Devices | iPhone and iPad, upright and on either side |
| Photos | Permission to add pictures (`NSPhotoLibraryAddUsageDescription`), asked at the first **Save to Photos** |
| Files | The app's `Documents` folder: the settings (`settings.ron`) and the exported pictures (`exports`) |
| StoreKit | In the App Store build: the product `app.fractique.pro`, the payment queue and the restore |

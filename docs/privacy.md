# Privacy

Fractique collects nothing. It has no account, no analytics, no advertising and no server of its own that the app
reports to. The pictures, the scenes and the settings stay on the device.

| What | Where it goes |
| --- | --- |
| Pictures you export | Where you send them: Photos, the gallery, a file, or the app chosen in the share sheet |
| Settings, the last session, the pictures you keep | The app's own storage on the device; deleting the app removes them |
| The count of free saves of the day (App Store builds) | The app's own storage on the device; on an iPhone and an iPad the device's Keychain too, where it stays when the app is deleted |
| The purchase of Fractique Pro (App Store builds) | Apple handles the payment; the app learns from the App Store only whether the Apple Account owns it, and keeps that on the device |
| Checking for updates (the desktop downloads from GitHub) | The app asks GitHub for the list of releases; GitHub sees the address the request comes from, as any website does. The App Store builds, the Android and the web app do not check |

The app asks for one permission, on a phone: to add the pictures you export to the photo library. It cannot read
the library.

`fractique-server`, the HTTP server you may run yourself, is yours: what it logs and keeps is described in
[server.md](server.md).

Questions: open an issue at <https://github.com/tarpetos/fractique/issues>.

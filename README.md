# Photos

**Your photos and videos, kept on your own devices instead of a cloud.**

**[Download](https://github.com/jachymri/uuob-releases)** for Android and Mac · Built for **[UUOB](https://github.com/UUOB/uuob)**

## Built for UUOB

Photos has no server and no account. Your photos live in [UUOB](https://github.com/UUOB/uuob), on your
own devices. They are end-to-end encrypted, so only your devices (and people you share with) can read them, and
UUOB keeps them in sync.

1. Install UUOB, then Photos.
2. Open Photos. UUOB asks *"Photos wants to use your network"*. Tap **Allow**.
3. Photos you add appear on all your linked devices.

No UUOB on this device? In UUOB on another device choose **Apps → Connect with a code**, and type the code
into Photos. [Full setup guide →](https://github.com/UUOB/uuob/blob/main/docs/setup.md)

**It saves space on your phone.** Make a Mac or a Linux server with plenty of space a *storage device*
(under **Devices** in UUOB). The phone then keeps small previews and lets go of an original only once the
storage device has it. Open the photo and it comes back.

## Features

- **Add photos and videos** from your device.
- **A grid by month**
- **Full size**: swipe between photos; videos play.
- **Recently Deleted**: restore deleted photos for 30 days.
- **Browse offline**: previews are always on the device.

## For developers

Photos is native on each system, over one Rust core, like UUOB itself:

```
core/            Photos without its screens (Rust): its data, rules and the calls to UUOB, for all three
                 systems; Swift and Kotlin reach it through UniFFI
android/         the Android app (Jetpack Compose, Material 3)
mac/             the Mac app (SwiftUI, macOS 14 or newer)
linux/           the Linux app (GTK 4 and Libadwaita 1.5, through Relm4)
sdk/             UUOB's SDK for apps, as a git submodule (UUOB-sdk): the client (sdk/client) and the
                 controls and connect screen every app shares (sdk/kit: android, mac, linux)
uuob/            UUOB's repository, as a git submodule: the build scripts (uuob/docker)
app/             the earlier web version (Svelte in Tauri), kept until the native one replaces it
```

```bash
git clone --recurse-submodules https://github.com/UUOB/photos.git && cd photos
```

Everything builds in UUOB's containers (`uuob/docker`; each builds its image on first use):

```bash
uuob/docker/linux.sh cargo build -p photos-linux     # the Linux app, target/debug/uuob-photos
uuob/docker/linux.sh cargo test                       # the core's tests
uuob/docker/android.sh uuob/docker/apk.sh             # a signed APK for arm64 phones, in dist/
gh workflow run mac.yml                               # the Mac app, built on GitHub (or mac/build.sh on a Mac)
```

On Linux, videos need GTK's GStreamer module and codecs (Debian and Ubuntu: `libgtk-4-media-gstreamer`,
`gstreamer1.0-libav`); without them videos are still kept, just without a preview.

To publish a new version, raise `version` in `Cargo.toml`, commit, push and run `uuob/docker/release.sh`
(see [UUOB for developers](https://github.com/UUOB/uuob/blob/main/docs/developers.md), *Updates*).

To use a newer SDK or UUOB: `git -C sdk pull origin main` (or `uuob`), then commit that folder.

## License

[GNU Affero General Public License, version 3 or later](LICENSE).

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

```
app/             the app (Svelte) and its shell (Tauri v2, in app/src-tauri)
uuob/            UUOB's repository, as a git submodule: the app SDK (uuob/ui/uuob.js and
                 uuob/crates/uuob-sdk), the connect screen and design tokens (uuob/ui),
                 in-app updates (uuob/crates/uuob-update) and the build scripts (uuob/docker)
```

```bash
git clone --recurse-submodules https://github.com/UUOB/photos.git && cd photos
```

Everything runs in UUOB's dev container (`uuob/docker/dev.sh` builds it on first use):

```bash
uuob/docker/dev.sh sh -c 'npm ci && npm run build'   # the front end
uuob/docker/dev.sh cargo build                       # the desktop app (Linux)
uuob/docker/android.sh uuob/docker/apk.sh            # a signed APK for arm64 phones, in dist/
gh workflow run mac.yml                              # the Mac app, built on GitHub
```

To publish a new version, raise `version` in `app/src-tauri/tauri.conf.json`, commit, push and run
`uuob/docker/release.sh` (see [UUOB for developers](https://github.com/UUOB/uuob/blob/main/docs/developers.md), *Updates*).

To use a newer UUOB SDK: `git -C uuob pull origin main`, then commit the `uuob` folder.

## License

[GNU Affero General Public License, version 3 or later](LICENSE).

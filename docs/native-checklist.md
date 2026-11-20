# Photos, native: what the Svelte version does (and the native one must)

Taken from `app/Photos.svelte` and UUOB's `ui/Gate.svelte`, `ui/Update.svelte`.

## Connecting (the shared gate)
- [x] Connected before: show the app at once (from cache), check in the background
- [x] UUOB not found: "Photos needs UUOB", "Looking for UUOB…", re-check every 3 s; Android: start UUOB once (`uuob://start`), "Open UUOB" button
- [ ] Or a code from UUOB on another device (`42-amber-river-falcon`, four parts) → linked through the tunnel
- [x] UUOB found, not connected: "Connect Photos", "UUOB is running on <device>", Connect (Android opens `uuob://connect`), "Use a code instead"
- [x] Waiting: "Allow it in UUOB", Cancel
- [x] Offline banner: "Waiting for UUOB. Your changes are kept and sent when it's back."
- [ ] "Answer in UUOB on one of your devices." while a question waits on a remote UUOB
- [x] Disconnected in UUOB (401) → back to the connect screen, cache cleared
- [ ] "Update to x.y.z" banner, checked at start and on return (at most hourly)

## Data (unchanged)
- [x] One container per month, name `YYYY-MM`, created with key `month<YYYYMM>00000` (same id on every device)
- [x] Collection `photos`: `{ taken, added, name, type, size, file, w?, h?, thumb?, duration?, deleted? }`, written without the `container` field
- [x] Original bytes via `/api/files` (`put`, `url`, `delete`)
- [x] `thumb`: JPEG data URL, short side ≤ 240 px (never upscaled), quality 0.6; video: frame at min(0.1 s, duration), plus `duration` (s), `w`, `h`
- [x] No preview possible → stored without one
- [x] `taken` = the file's modified time, or now

## Screens
- [x] Header "Photos" + count; Recently Deleted button only when the trash isn't empty; Add (disabled while adding)
- [x] Grid grouped by month of `taken` ("November 2026"), newest first, square cells (~150 px min), preview or a photo/play glyph
- [x] Video badge with length `m:ss`
- [x] Empty: "No photos yet", "Add photos and videos from this device. They are kept in UUOB, on your own devices.", Add photos
- [x] Picker: images and videos, several at once; "Adding n of N…"; per-file error "<name> could not be added. <reason>"
- [x] Viewer: date ("Nov 19, 2026, 9:30 AM"), close, preview first then the original; video plays with controls (poster = preview) (Linux and Android, with an H.264 and a VP8 test video)
- [x] Viewer: previous/next buttons, arrow keys, Esc, swipe (> 60 px, not on video)
- [ ] Original unreachable: "The original is on a storage device that can't be reached right now."
- [x] Delete (viewer) → `deleted: now`, moves to the next photo or closes
- [x] Recently Deleted: back, "Photos stay here for 30 days after you delete them, then they are removed from all your devices.", badge "n days", newest deletion first, empty "Nothing here"
- [x] In trash viewer: Restore (removes `deleted`), Delete → confirm "Delete this photo/video?" "It is removed from all your devices. This can't be undone." → file then record deleted
- [x] On first load, anything deleted > 30 days ago is destroyed
- [x] Phones: each month container with `keep: 'all'` → `keep: 'list'` (once per container per run)
- [x] Live: changes from other devices show up

## How it was checked (2026-11-19)
- Linux (Xvfb, against a real UUOB with seeded data): connect screen, code, Cancel, grid, viewer, arrows, delete, Recently Deleted, Restore, adding from a new month, error message; records read back from UUOB match the web version's shape.
- Android emulator: old web Photos 0.3.0 wrote a photo, the native 0.4.0 installed over it (same key, higher versionCode), asked for Allow once, showed it; picker, viewer, swipe, delete, trash, Delete forever; the old app put back read what the native one wrote. UUOB recorded `app.uuob.photos` for its connection.
- Videos: preview frame and length on Linux and Android (Linux needs GTK's GStreamer module, `libgtk-4-media-gstreamer`, and `gstreamer1.0-libav` for H.264).
- Not checked on a device (left unticked): the update banner (no newer feed), the "original unreachable" and "answer on another device" notes (need a second device), linking with a code to a UUOB on another device (the SDK and the screen support it), the Mac app (written in the web look; needs a Mac to build).

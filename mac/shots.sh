#!/usr/bin/env bash
# Screenshots of Photos on the Mac, for checking how it looks (mac.yml):
# connected to a UUOB with demo pictures (macOS's own desktop pictures); the
# grid, a photo open with a button's hint, its Share menu under the button, its
# right-click menu, Info and Settings, light and dark; then a small window.
#   mac/shots.sh <Photos.app> <uuob binary> <seeded data dir> <out dir>
set -uo pipefail
name=Photos exe=$1/Contents/MacOS/Photos UUOB=$2 DATA=$3 out=$4
mkdir -p "$out"
. "$(dirname "$0")/../sdk/kit/mac/shots-lib.sh"

mode=start
uuob_up
app_up
for mode in light dark; do
  [ $mode = dark ] && { se 'tell appearance preferences to set dark mode to true'; sleep 2; }
  shot grid
  if click "~Photo from"; then
    sleep 2; shot viewer
    hover "Info (⌘I)" && shot viewer-hint
    # Share hangs under its button.
    click "Share" && { sleep 2; shot share; esc; }
    # Right-click on the photo: the grid's menu.
    pointer 560 420 right; shot viewer-menu; esc
    cmd i; shot info; esc
    esc
  fi
  hover "Hide sidebar (⌃⌘S)" && shot sidebar-hint
  cmd ","; shot settings
  se "tell process \"$name\" to click at {1040, 700}"; sleep 1.5; shot outside-click
  esc
done

mode=small
se "tell process \"$name\" to set size of front window to {520, 620}"; sleep 2
shot grid
app_down
uuob_down

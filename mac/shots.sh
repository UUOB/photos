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
  # The first photo (the grid starts right of the 240-wide sidebar, under the title).
  pointer 360 250 left; sleep 3
  if true; then
    shot viewer
    # The bar's icons sit at the window's top right: info, share, copy, save, delete.
    pointer 806 83; sleep 1; shot viewer-hint
    # Share hangs under its button.
    pointer 850 83 left; sleep 2; shot share; esc
    # Right-click on the photo: the grid's menu.
    pointer 500 400 right; shot viewer-menu; esc
    cmd i; shot info; esc
    esc
  fi
  # The toggle's hint (an icon has no words to find; the window sits at 12,31, 1000 wide, 680 high).
  pointer 222 681; sleep 1; shot sidebar-hint
  cmd ","; shot settings
  # Beside the sheet, on the window it covers (the screen is 1024 wide).
  win "click at {70, 640}"; sleep 1.5; shot outside-click
  esc
done

mode=small
win "set size of front window to {520, 620}"; sleep 2
shot grid
app_down
uuob_down

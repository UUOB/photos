#!/usr/bin/env bash
# On a Mac (GitHub's Mac runner, see .github/workflows/mac.yml): build
# Photos.app for Apple silicon, then the .dmg (first install) and
# the .app.tar.gz (what the Update button installs), in out/.
# Needs Rust with the aarch64-apple-darwin target, and Xcode.
set -euo pipefail
cd "$(dirname "$0")/.."
version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
build=mac/build gen=mac/build/gen app=mac/build/Photos.app
rm -rf "$build" && mkdir -p "$gen" out

# 1. The core (with UUOB's SDK), for Apple silicon, as one library.
MACOSX_DEPLOYMENT_TARGET=14.0 cargo build --release --lib -p photos-core --target aarch64-apple-darwin
cp target/aarch64-apple-darwin/release/libphotos_core.dylib "$build/libphotos_core.dylib"
install_name_tool -id @rpath/libphotos_core.dylib "$build/libphotos_core.dylib"

# 2. Its Swift bindings (UniFFI): Photos' core and UUOB's SDK.
cargo run -q -p photos-core --features bindgen --bin uniffi-bindgen -- \
  generate --library target/aarch64-apple-darwin/release/libphotos_core.dylib --language swift --out-dir "$gen"

# 3. The app, for Apple silicon: its screens, UUOB's kit for apps, the bindings.
maps=(); for m in "$gen"/*.modulemap; do maps+=(-Xcc "-fmodule-map-file=$m"); done
for arch in arm64; do
  swiftc -O -parse-as-library -swift-version 5 -target "$arch-apple-macos14.0" -module-name Photos \
    -I "$gen" "${maps[@]}" \
    mac/Sources/*.swift sdk/kit/mac/*.swift "$gen"/*.swift \
    -L "$build" -lphotos_core -Xlinker -rpath -Xlinker @executable_path/../Frameworks \
    -o "$build/Photos-$arch"
done

# 4. Photos.app, signed for this Mac only (no paid Apple account: macOS asks once, "Open Anyway").
mkdir -p "$app/Contents/MacOS" "$app/Contents/Frameworks" "$app/Contents/Resources"
cp "$build/Photos-arm64" "$app/Contents/MacOS/Photos"
cp "$build/libphotos_core.dylib" "$app/Contents/Frameworks/"
cp mac/Photos.icns "$app/Contents/Resources/"
sed "s/VERSION/$version/g" mac/Info.plist > "$app/Contents/Info.plist"
codesign --force --deep --sign - "$app"

# 5. What gets published (the same names as before, so updates find them).
mkdir -p "$build/dmg" && cp -R "$app" "$build/dmg/" && ln -s /Applications "$build/dmg/Applications"
hdiutil create -volname Photos -srcfolder "$build/dmg" -ov -format UDZO "out/Photos_${version}_arm64.dmg"
(cd "$build" && COPYFILE_DISABLE=1 tar -czf "../../out/Photos.app.tar.gz" Photos.app)
ls -la out

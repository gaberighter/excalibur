#!/bin/sh
# Builds Excalibur.app with its helper, plugins and themes inside, and zips
# it for a release. Run from anywhere; needs the Rust toolchain and tauri-cli.
set -eu
cd "$(dirname "$0")/.."

# The helper is a resource of the app, so it has to exist before bundling.
cargo build --release -p stats-probe
cargo tauri build --config src-tauri/tauri.bundle.conf.json

app="target/release/bundle/macos/Excalibur.app"
version=$(sed -n 's/^ *"version": "\(.*\)",$/\1/p' src-tauri/tauri.conf.json)
archive="target/release/Excalibur-$version-$(uname -m).zip"
rm -f "$archive"
ditto -c -k --keepParent "$app" "$archive"

echo
echo "app:     $app"
echo "archive: $archive"
echo "sha256:  $(shasum -a 256 "$archive" | cut -d ' ' -f 1)"

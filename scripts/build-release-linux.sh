#!/bin/bash
# Builds YuE2 Studio for Linux x86-64: a .deb and an AppImage with the pinned
# yue2.cpp engine on Vulkan and CUDA 12 (CUDA 13 too when YUE_CUDA13=1) and the
# audio-to-MIDI transcriber bundled. Model weights are downloaded in the app.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
engine="$root/desktop/src-tauri/resources/yue2-cpp"
commit="$(sed -n 's/.*"commit": "\(.*\)".*/\1/p' "$root/engines/yue2-cpp-source.json")"

if [ ! -x "$engine/yue-server" ] || ! grep -q "$commit" "$engine/runtime.json" 2>/dev/null; then
    rm -rf "$engine"
    "$root/scripts/build-yue-runtime-linux.sh" "$engine"
fi

midi="$root/desktop/src-tauri/resources/music-midi"
midi_commit="$(sed -n 's/.*"commit": "\(.*\)".*/\1/p' "$root/engines/music-midi-source.json")"
if [ ! -x "$midi/music-midi" ] || [ ! -x "$midi/music-caption" ] || [ "$(cat "$midi/runtime.commit" 2>/dev/null)" != "$midi_commit" ]; then
    rm -rf "$midi"
    "$root/scripts/build-midi-runtime-linux.sh" "$midi"
fi

cd "$root"
# a slow link reads a tarball for minutes: give npm the time and retries, and reuse what the cache holds
npm --prefix app ci --no-audit --no-fund --prefer-offline --fetch-retries=5 --fetch-retry-maxtimeout=120000 --fetch-timeout=900000
npm --prefix desktop ci --no-audit --no-fund --prefer-offline --fetch-retries=5 --fetch-retry-maxtimeout=120000 --fetch-timeout=900000
cd desktop
# the display driver's libcuda and the CUDA libraries the studio downloads are never packed
export LINUXDEPLOY_EXCLUDED_LIBRARIES="libcuda.so*;libcudart.so*;libcublas.so*;libcublasLt.so*"
npm exec tauri build -- --config src-tauri/tauri.linux-release.conf.json --bundles deb,appimage

find src-tauri/target/release/bundle -name '*.deb' -o -name '*.AppImage'

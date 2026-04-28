#!/usr/bin/env bash
set -euo pipefail

CRATE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORKSPACE_DIR="$(cd "${CRATE_DIR}/.." && pwd)"
TARGET_DIR="${WORKSPACE_DIR}/target"
OUT_DIR="${CRATE_DIR}/build/AdventermFFI.xcframework"

# Build all four iOS-family targets in release.
for target in aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios; do
    rustup target add "$target" >/dev/null
    cargo build --release -p adventerm_ffi --target "$target"
done

# Fat archive for the simulator slice (arm64 + x86_64).
mkdir -p "${TARGET_DIR}/ios-sim-fat/release"
lipo -create \
    "${TARGET_DIR}/aarch64-apple-ios-sim/release/libadventerm_ffi.a" \
    "${TARGET_DIR}/x86_64-apple-ios/release/libadventerm_ffi.a" \
    -output "${TARGET_DIR}/ios-sim-fat/release/libadventerm_ffi.a"

# Bundle the XCFramework.
rm -rf "$OUT_DIR"
xcodebuild -create-xcframework \
    -library "${TARGET_DIR}/aarch64-apple-ios/release/libadventerm_ffi.a" \
    -headers "${CRATE_DIR}/include" \
    -library "${TARGET_DIR}/ios-sim-fat/release/libadventerm_ffi.a" \
    -headers "${CRATE_DIR}/include" \
    -output "$OUT_DIR"

echo "Built $OUT_DIR"

#!/usr/bin/env bash
# Compila reader-ffi para iOS e monta XCFramework + bindings Swift (requer macOS + Xcode).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "Este script precisa de macOS + Xcode." >&2
  exit 1
fi

if ! command -v xcodebuild >/dev/null; then
  echo "xcodebuild não encontrado. Instale o Xcode." >&2
  exit 1
fi

IOS_DIR="${ROOT}/platforms/ios"
OUT_DIR="${IOS_DIR}/build"
FRAMEWORK_NAME="ReaderFfiFFI"
XCFRAMEWORK="${OUT_DIR}/${FRAMEWORK_NAME}.xcframework"
SWIFT_OUT="${IOS_DIR}/Sources/ReaderFfi"

mkdir -p "${OUT_DIR}" "${SWIFT_OUT}"

TARGETS=(
  "aarch64-apple-ios"
  "aarch64-apple-ios-sim"
  "x86_64-apple-ios"
)

for target in "${TARGETS[@]}"; do
  echo "==> rustup target add ${target}"
  rustup target add "${target}" >/dev/null
  echo "==> cargo build -p reader-ffi --release --target ${target}"
  cargo build -p reader-ffi --release --target "${target}"
done

# Generate Swift bindings from a host build.
echo "==> generating Swift bindings"
cargo build -p reader-ffi
HOST_LIB="${ROOT}/target/debug/libreader_ffi.dylib"
test -f "${HOST_LIB}"
cargo run -p reader-ffi --features bindgen --bin uniffi-bindgen -- \
  generate --library "${HOST_LIB}" \
  --language swift \
  --out-dir "${OUT_DIR}/swift-gen" \
  --config "${ROOT}/crates/reader-ffi/uniffi.toml" \
  --no-format

# UniFFI emits ReaderFfi.swift + ReaderFfiFFI.h + ReaderFfiFFI.modulemap (names from uniffi.toml).
cp "${OUT_DIR}/swift-gen/"*.swift "${SWIFT_OUT}/"
HEADER="${OUT_DIR}/swift-gen/ReaderFfiFFI.h"
MODULEMAP="${OUT_DIR}/swift-gen/ReaderFfiFFI.modulemap"
test -f "${HEADER}"
test -f "${MODULEMAP}"

make_fw() {
  local target="$1"
  local slice_dir="$2"
  local lib="${ROOT}/target/${target}/release/libreader_ffi.a"
  test -f "${lib}"
  local fw="${OUT_DIR}/${slice_dir}/${FRAMEWORK_NAME}.framework"
  rm -rf "${fw}"
  mkdir -p "${fw}/Headers" "${fw}/Modules"
  cp "${lib}" "${fw}/${FRAMEWORK_NAME}"
  cp "${HEADER}" "${fw}/Headers/"
  cp "${MODULEMAP}" "${fw}/Modules/module.modulemap"
  cat > "${fw}/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleIdentifier</key>
  <string>com.readerengine.${FRAMEWORK_NAME}</string>
  <key>CFBundleName</key>
  <string>${FRAMEWORK_NAME}</string>
  <key>CFBundleVersion</key>
  <string>1</string>
  <key>CFBundleShortVersionString</key>
  <string>1.0</string>
  <key>CFBundlePackageType</key>
  <string>FMWK</string>
  <key>MinimumOSVersion</key>
  <string>15.0</string>
</dict>
</plist>
EOF
}

make_fw "aarch64-apple-ios" "ios-device"
make_fw "aarch64-apple-ios-sim" "ios-sim-arm64"
make_fw "x86_64-apple-ios" "ios-sim-x64"

# Merge simulator slices into one fat binary framework for XCFramework.
SIM_MERGED="${OUT_DIR}/ios-sim-merged/${FRAMEWORK_NAME}.framework"
rm -rf "${SIM_MERGED}"
mkdir -p "${SIM_MERGED}/Headers" "${SIM_MERGED}/Modules"
cp "${OUT_DIR}/ios-sim-arm64/${FRAMEWORK_NAME}.framework/Headers/"* "${SIM_MERGED}/Headers/"
cp "${OUT_DIR}/ios-sim-arm64/${FRAMEWORK_NAME}.framework/Modules/module.modulemap" "${SIM_MERGED}/Modules/"
cp "${OUT_DIR}/ios-sim-arm64/${FRAMEWORK_NAME}.framework/Info.plist" "${SIM_MERGED}/"
lipo -create \
  "${OUT_DIR}/ios-sim-arm64/${FRAMEWORK_NAME}.framework/${FRAMEWORK_NAME}" \
  "${OUT_DIR}/ios-sim-x64/${FRAMEWORK_NAME}.framework/${FRAMEWORK_NAME}" \
  -output "${SIM_MERGED}/${FRAMEWORK_NAME}"

rm -rf "${XCFRAMEWORK}"
xcodebuild -create-xcframework \
  -framework "${OUT_DIR}/ios-device/${FRAMEWORK_NAME}.framework" \
  -framework "${SIM_MERGED}" \
  -output "${XCFRAMEWORK}"

ZIP="${OUT_DIR}/${FRAMEWORK_NAME}.xcframework.zip"
rm -f "${ZIP}"
(
  cd "${OUT_DIR}"
  zip -r -y "${FRAMEWORK_NAME}.xcframework.zip" "${FRAMEWORK_NAME}.xcframework"
)

CHECKSUM="$(swift package compute-checksum "${ZIP}" 2>/dev/null || shasum -a 256 "${ZIP}" | awk '{print $1}')"
echo "OK: ${XCFRAMEWORK}"
echo "ZIP: ${ZIP}"
echo "checksum: ${CHECKSUM}"
echo "Swift sources: ${SWIFT_OUT}"
echo "Atualize platforms/ios/Package.swift (binaryTarget url/checksum) no release."

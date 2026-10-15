#!/usr/bin/env bash
# Gera apenas as bindings Kotlin (host), sem NDK.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
OUT_DIR="${ROOT}/platforms/android/reader-engine/src/main/java"
cargo build -p reader-ffi
case "$(uname -s)" in
  Darwin) LIB="${ROOT}/target/debug/libreader_ffi.dylib" ;;
  *) LIB="${ROOT}/target/debug/libreader_ffi.so" ;;
esac
test -f "${LIB}"
cargo run -p reader-ffi --features bindgen --bin uniffi-bindgen -- \
  generate --library "${LIB}" \
  --language kotlin \
  --out-dir "${OUT_DIR}" \
  --config "${ROOT}/crates/reader-ffi/uniffi.toml" \
  --no-format
echo "Bindings em platforms/android/reader-engine/src/main/java/uniffi/reader_ffi/"

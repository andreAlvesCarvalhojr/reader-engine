#!/usr/bin/env bash
# Gera apenas as bindings Kotlin (host), sem NDK.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
cargo build -p reader-ffi
cargo run -p reader-ffi --features bindgen --bin uniffi-bindgen -- \
  generate --library "${ROOT}/target/debug/libreader_ffi.so" \
  --language kotlin \
  --out-dir "${ROOT}/platforms/android/app/src/main/java" \
  --config "${ROOT}/crates/reader-ffi/uniffi.toml" \
  --no-format
echo "Bindings em platforms/android/app/src/main/java/uniffi/reader_ffi/"

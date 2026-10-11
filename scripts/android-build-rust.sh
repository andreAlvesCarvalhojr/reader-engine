#!/usr/bin/env bash
# Compila reader-ffi para ABIs Android e gera bindings Kotlin.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

ABIS=("arm64-v8a" "armeabi-v7a" "x86_64")
TARGETS=("aarch64-linux-android" "armv7-linux-androideabi" "x86_64-linux-android")

if ! command -v cargo >/dev/null; then
  echo "cargo não encontrado. Instale Rust: https://rustup.rs" >&2
  exit 1
fi

if [[ -z "${ANDROID_NDK_HOME:-}" && -z "${NDK_HOME:-}" ]]; then
  if [[ -d "${ANDROID_HOME:-}/ndk" ]]; then
    ANDROID_NDK_HOME="$(ls -d "${ANDROID_HOME}/ndk"/* | sort -V | tail -1)"
  fi
fi
export ANDROID_NDK_HOME="${ANDROID_NDK_HOME:-${NDK_HOME:-}}"

if [[ -z "${ANDROID_NDK_HOME}" || ! -d "${ANDROID_NDK_HOME}" ]]; then
  echo "NDK não encontrado." >&2
  echo "Instale via Android Studio → SDK Manager → NDK, depois:" >&2
  echo "  export ANDROID_HOME=\"\$HOME/Library/Android/sdk\"   # Mac" >&2
  echo "  ls \"\$ANDROID_HOME/ndk\"                             # veja a pasta da versão" >&2
  echo "  export ANDROID_NDK_HOME=\"\$ANDROID_HOME/ndk/27.0.12077973\"" >&2
  echo "Ou só exporte ANDROID_HOME; este script usa o NDK mais recente automaticamente." >&2
  exit 1
fi

echo "==> ANDROID_NDK_HOME=${ANDROID_NDK_HOME}"

API_LEVEL="${ANDROID_API_LEVEL:-24}"
HOST_TAG="$(uname -s | tr '[:upper:]' '[:lower:]')-x86_64"
TOOLCHAIN="${ANDROID_NDK_HOME}/toolchains/llvm/prebuilt/${HOST_TAG}"
if [[ ! -d "${TOOLCHAIN}" ]]; then
  # Apple Silicon hosts
  HOST_TAG="$(uname -s | tr '[:upper:]' '[:lower:]')-aarch64"
  TOOLCHAIN="${ANDROID_NDK_HOME}/toolchains/llvm/prebuilt/${HOST_TAG}"
fi

for i in "${!TARGETS[@]}"; do
  target="${TARGETS[$i]}"
  abi="${ABIS[$i]}"
  rustup target add "${target}" >/dev/null

  case "${target}" in
    aarch64-linux-android) cc_prefix="aarch64-linux-android" ;;
    armv7-linux-androideabi) cc_prefix="armv7a-linux-androideabi" ;;
    x86_64-linux-android) cc_prefix="x86_64-linux-android" ;;
  esac

  export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="${TOOLCHAIN}/bin/${cc_prefix}${API_LEVEL}-clang"
  export CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_LINKER="${TOOLCHAIN}/bin/armv7a-linux-androideabi${API_LEVEL}-clang"
  export CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER="${TOOLCHAIN}/bin/${cc_prefix}${API_LEVEL}-clang"
  export CC_${target//-/_}="${TOOLCHAIN}/bin/${cc_prefix}${API_LEVEL}-clang"
  export AR_${target//-/_}="${TOOLCHAIN}/bin/llvm-ar"

  echo "==> building ${target} (${abi})"
  cargo build -p reader-ffi --release --target "${target}"

  out_dir="${ROOT}/platforms/android/app/src/main/jniLibs/${abi}"
  mkdir -p "${out_dir}"
  cp "${ROOT}/target/${target}/release/libreader_ffi.so" "${out_dir}/libreader_ffi.so"
done

echo "==> generating Kotlin bindings"
"${ROOT}/scripts/generate-kotlin-bindings.sh"

echo "OK: jniLibs + bindings atualizados."

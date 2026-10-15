#!/usr/bin/env bash
# Compila reader-wasm e gera platforms/web/pkg para o leitor estático.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

if ! command -v wasm-pack >/dev/null; then
  echo "wasm-pack nao encontrado. Instale: cargo install wasm-pack" >&2
  echo "Ou: curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh" >&2
  exit 1
fi

rustup target add wasm32-unknown-unknown >/dev/null

echo "==> wasm-pack build -p reader-wasm"
wasm-pack build crates/reader-wasm \
  --target web \
  --out-dir "$ROOT/platforms/web/pkg" \
  --release

# wasm-pack gera .gitignore dentro de pkg; ok.
rm -f "$ROOT/platforms/web/pkg/.gitignore"

echo "OK: platforms/web/pkg"
echo "Sirva a pasta (WASM exige HTTP):"
echo "  python3 -m http.server 8788 --directory platforms/web"
echo "  # abre http://127.0.0.1:8788"

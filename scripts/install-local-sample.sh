#!/usr/bin/env bash
# Copia fixtures/local/*.json|*.epub para assets Android (gitignored).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC="${1:-$ROOT/fixtures/local}"
DEST="$ROOT/platforms/android/app/src/main/assets/samples/local"

if [[ ! -d "$SRC" ]]; then
  echo "Pasta nao encontrada: $SRC" >&2
  echo "Coloque o arquivo convertido em fixtures/local/lessons-pt.json" >&2
  echo "Ex.: cp /caminho/lessons-pt.json fixtures/local/" >&2
  exit 1
fi

mkdir -p "$DEST"
copied=0
shopt -s nullglob
for f in "$SRC"/*.{json,epub}; do
  base="$(basename "$f")"
  # skip raw originals
  [[ "$base" == *.raw.json ]] && continue
  cp -f "$f" "$DEST/$base"
  echo "OK: $DEST/$base"
  copied=$((copied + 1))
done

if [[ "$copied" -eq 0 ]]; then
  echo "Nenhum .json/.epub em $SRC" >&2
  exit 1
fi

echo "Reinicie/reinstale o app Android para ver o chip da amostra."

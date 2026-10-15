# Web (WASM estático)

Leitor estático no browser: Rust via `wasm-bindgen` (`crates/reader-wasm`), UI em HTML/JS sem servidor de app.

## Build

Na raiz do monorepo:

```bash
./scripts/build-web.sh
```

Isso gera `platforms/web/pkg/` (não versionado).

## Rodar

WASM não carrega via `file://`. Sirva a pasta:

```bash
python3 -m http.server 8788 --directory platforms/web
# http://127.0.0.1:8788
```

## Uso

- Amostras embutidas (`samples/noite-clara.json`, `samples/sample.epub`)
- **Abrir arquivo** — picker local (`.epub` / `.json` / `.hpub`)
- Tema, escala, sumário, busca, bookmarks (bookmarks no `localStorage`)

A engine sempre recebe `(nome, bytes)` do host — igual ao Android/FFI.

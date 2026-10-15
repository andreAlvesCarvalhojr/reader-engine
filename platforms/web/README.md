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

## Uso da UI demo

- Amostras embutidas (`samples/noite-clara.json`, `samples/sample.epub`)
- **Abrir arquivo** — picker local (`.epub` / `.json` / `.hpub`)
- Tema, escala, sumário, busca, bookmarks (bookmarks no `localStorage`)

## Integrar no seu JS

```js
import init, { WasmReaderEngine } from "./pkg/reader_wasm.js";

await init();
const engine = new WasmReaderEngine();
const session = engine.open("livro.epub", bytes); // Uint8Array
iframe.srcdoc = session.currentHtml();
```

A engine sempre recebe `(nome, bytes)` do host — igual ao Android/FFI. Guia completo: [README raiz](../../README.md#como-usar--web-wasm).

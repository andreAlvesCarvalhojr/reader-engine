# reader-engine

Engine de leitura headless em Rust (EPUB, HPUB, JSON próprio e formatos customizados) com UI nativa por plataforma.

```
crates/
  reader-core   modelo Publication, trait FormatParser, Registry, ReaderSession, ThemeTokens
  reader-epub   parser .epub
  reader-hpub   parser .hpub (Baker)
  reader-json   parser JSON próprio
  reader-demo   interface web de demonstração (Axum)
  reader-ffi    UniFFI (Kotlin/Swift) — app envia (nome, bytes)
  reader-wasm   wasm-bindgen — leitor web estático
themes/         tokens: material3 | hig | fluent
platforms/
  android/      Compose + WebView (amostras + picker de arquivo)
  web/          UI estática + WASM (sem servidor de app)
```

## Desenvolvimento

```bash
cargo test --workspace
cargo run -p reader-demo
# abre http://127.0.0.1:8787
```

### Android (FFI + WebView)

A engine **não abre arquivos sozinha**. O app lê bytes (asset ou picker) e chama `ReaderEngine.open(name, bytes)`.

```bash
./scripts/generate-kotlin-bindings.sh   # regenera Kotlin
./scripts/android-build-rust.sh         # precisa NDK → jniLibs/*.so
# depois abra platforms/android no Android Studio
```

Detalhes: [`platforms/android/README.md`](platforms/android/README.md).

### Web estática (WASM)

```bash
./scripts/build-web.sh
python3 -m http.server 8788 --directory platforms/web
# http://127.0.0.1:8788
```

Requer `wasm-pack` e target `wasm32-unknown-unknown`. Detalhes: [`platforms/web/README.md`](platforms/web/README.md).

Requer Rust 1.85+ (`rust-toolchain.toml`).

## Formato customizado
Implemente `FormatParser` e registre:

```rust
let mut reg = Registry::new();
reg.register(Box::new(EpubParser)).register(Box::new(MyParser));
let publication = reg.open(Source { name: "livro.xyz".into(), bytes })?;
```

## Próximos passos
1. iOS (SwiftUI + WebView) reutilizando `reader-ffi`.
2. Empacotar `platforms/web` (PWA / CDN) a partir do `pkg/` gerado.
3. Mais fixtures e ContentHost compartilhado.

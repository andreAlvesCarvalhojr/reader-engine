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
themes/         tokens: material3 | hig | fluent
platforms/
  android/      Compose + WebView (amostras + picker de arquivo)
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

Requer Rust 1.85+ (`rust-toolchain.toml`).

## Formato customizado
Implemente `FormatParser` e registre:

```rust
let mut reg = Registry::new();
reg.register(Box::new(EpubParser)).register(Box::new(MyParser));
let publication = reg.open(Source { name: "livro.xyz".into(), bytes })?;
```

## Próximos passos
1. `reader-wasm` (wasm-bindgen) para web sem servidor.
2. iOS (SwiftUI + WebView) reutilizando `reader-ffi`.
3. Mais fixtures e ContentHost compartilhado.

# reader-engine

Engine de leitura headless em Rust (EPUB, HPUB, JSON próprio e formatos customizados) com UI nativa por plataforma.

```
crates/
  reader-core   modelo Publication, trait FormatParser, Registry, ReaderSession, ThemeTokens
  reader-epub   parser .epub
  reader-hpub   parser .hpub (Baker)
  reader-json   parser JSON próprio
  reader-demo   interface web de demonstração (Axum)
themes/         tokens: material3 | hig | fluent
platforms/      UI nativa (Compose+M3, SwiftUI+HIG, WinUI3+Fluent, Web Components)
```

## Desenvolvimento

```bash
cargo test --workspace
cargo run -p reader-demo
# abre http://127.0.0.1:8787
```

Requer Rust 1.85+ (`rust-toolchain.toml`).

## Formato customizado
Implemente `FormatParser` e registre:

```rust
let mut reg = Registry::new();
reg.register(Box::new(EpubParser)).register(Box::new(MyParser));
let publication = reg.open(Source { name: "livro.xyz".into(), bytes })?;
```

## Próximos passos
1. Crate `reader-ffi` com UniFFI (Kotlin/Swift/C#) e `reader-wasm` (wasm-bindgen).
2. `ContentHost` nativo (WebView) + UIs em `platforms/`.

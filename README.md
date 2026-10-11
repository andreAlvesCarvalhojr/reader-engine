# reader-engine

Engine de leitura headless em Rust (EPUB, HPUB, JSON próprio e formatos customizados) com UI nativa por plataforma.

```
crates/
  reader-core   modelo Publication, trait FormatParser, Registry, ReaderSession, ThemeTokens
  reader-epub   parser .epub
  reader-hpub   parser .hpub (Baker)
  reader-json   parser JSON próprio
themes/         tokens: material3 | hig | fluent
platforms/      UI nativa (Compose+M3, SwiftUI+HIG, WinUI3+Fluent, Web Components)
```

## Formato customizado
Implemente `FormatParser` e registre:

```rust
let mut reg = Registry::new();
reg.register(Box::new(EpubParser)).register(Box::new(MyParser));
let publication = reg.open(Source { name: "livro.xyz".into(), bytes })?;
```

## Próximos passos
1. `cargo build` / `cargo test` (código ainda não compilado) e testes com fixtures.
2. Crate `reader-ffi` com UniFFI (Kotlin/Swift/C#) e `reader-wasm` (wasm-bindgen).
3. `ContentHost` nativo (WebView) + UIs em `platforms/`.

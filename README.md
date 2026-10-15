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
  android/      :reader-engine (AAR library) + demo app
  ios/          Swift Package + XCFramework
  web/          UI estática + WASM
```

## Desenvolvimento

```bash
cargo test --workspace
cargo run -p reader-demo
# abre http://127.0.0.1:8787
```

### Android library (AAR)

```bash
./scripts/android-build-rust.sh
cd platforms/android
./gradlew :reader-engine:assembleRelease
```

Consumo em outro app (GitHub Packages, após tag `vX.Y.Z`):

```kotlin
implementation("com.github.andreAlvesCarvalhojr:reader-engine:0.1.0")
```

Detalhes: [`platforms/android/README.md`](platforms/android/README.md).

### iOS (Swift Package)

```bash
# macOS + Xcode
./scripts/ios-build-xcframework.sh
# abra platforms/ios no Xcode como pacote local
```

Detalhes: [`platforms/ios/README.md`](platforms/ios/README.md).

### Web estática (WASM)

```bash
./scripts/build-web.sh
python3 -m http.server 8788 --directory platforms/web
# http://127.0.0.1:8788
```

Detalhes: [`platforms/web/README.md`](platforms/web/README.md).

Requer Rust 1.85+ (`rust-toolchain.toml`).

## Formato customizado
Implemente `FormatParser` e registre:

```rust
let mut reg = Registry::new();
reg.register(Box::new(EpubParser)).register(Box::new(MyParser));
let publication = reg.open(Source { name: "livro.xyz".into(), bytes })?;
```

## Publicação mobile

Push de tag `v*`:

- AAR → GitHub Packages + asset no Release
- XCFramework zip → asset no Release (job macOS)

Workflow: `.github/workflows/publish-mobile.yml`.

## Próximos passos
1. App demo iOS (SwiftUI + WKWebView) usando o pacote.
2. Empacotar `platforms/web` (PWA / CDN).
3. Mais fixtures e ContentHost compartilhado.

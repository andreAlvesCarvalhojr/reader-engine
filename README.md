# reader-engine

Engine de leitura headless em Rust (EPUB, HPUB, JSON próprio e formatos customizados), exposta como library para **Android**, **iOS** e **Web**.

Contrato em todas as plataformas: o **app fornece `(nome, bytes)`** — a engine não lê o filesystem.

Release atual: [**v0.1.0**](https://github.com/andreAlvesCarvalhojr/reader-engine/releases/tag/v0.1.0)

```
crates/
  reader-core / reader-epub / reader-hpub / reader-json
  reader-ffi     UniFFI (Kotlin + Swift)
  reader-wasm    wasm-bindgen (browser)
  reader-demo    demo HTTP (Axum)
platforms/
  android/       AAR (:reader-engine) + app demo
  ios/           Swift Package + XCFramework
  web/           UI estática + WASM
```

---

## Como usar — Android

### 1. Adicione o repositório Maven (GitHub Packages)

Em `settings.gradle.kts` (ou `settings.gradle`):

```kotlin
dependencyResolutionManagement {
    repositories {
        google()
        mavenCentral()
        maven {
            url = uri("https://maven.pkg.github.com/andreAlvesCarvalhojr/reader-engine")
            credentials {
                username = providers.gradleProperty("gpr.user").orNull
                    ?: System.getenv("GITHUB_ACTOR")
                password = providers.gradleProperty("gpr.key").orNull
                    ?: System.getenv("GITHUB_TOKEN")
            }
        }
    }
}
```

Crie um [PAT](https://github.com/settings/tokens) com scope `read:packages` e coloque no `~/.gradle/gradle.properties`:

```properties
gpr.user=SEU_USUARIO_GITHUB
gpr.key=SEU_PAT
```

### 2. Dependência

Em `app/build.gradle.kts`:

```kotlin
dependencies {
    implementation("com.github.andreAlvesCarvalhojr:reader-engine:0.1.0")
}
```

### 3. Alternativa sem Packages — AAR do Release

1. Baixe [`reader-engine-release.aar`](https://github.com/andreAlvesCarvalhojr/reader-engine/releases/download/v0.1.0/reader-engine-release.aar)
2. Coloque em `app/libs/`
3. Dependências:

```kotlin
dependencies {
    implementation(files("libs/reader-engine-release.aar"))
    implementation("net.java.dev.jna:jna:5.15.0@aar")
}
```

### 4. Código no app

```kotlin
import uniffi.reader_ffi.ReaderEngine
import android.webkit.WebView

fun openBook(webView: WebView, fileName: String, bytes: ByteArray) {
    val session = ReaderEngine().open(fileName, bytes)
    val html = session.currentHtml()
    webView.loadDataWithBaseURL(null, html, "text/html", "utf-8", null)

    // Navegação
    // session.next() / session.prev() / session.goTo(index)
    // session.setFontScale(1.2f)
    // session.setThemeJson(themeJson)
    // session.search("texto")
}
```

Os bytes vêm de `assets`, Storage Access Framework, download, etc.

Mais detalhes: [`platforms/android/README.md`](platforms/android/README.md).

---

## Como usar — iOS

### 1. Opção rápida — pacote local (desenvolvimento)

No Mac, na raiz deste repo:

```bash
./scripts/ios-build-xcframework.sh
```

No Xcode do **seu app**:

1. File → Add Package Dependencies…
2. Add Local… → selecione `platforms/ios`
3. Adicione o produto **ReaderEngine** ao target

### 2. Opção release — binary do GitHub

1. No seu `Package.swift` (ou Package Dependencies do Xcode), use o XCFramework publicado:

```swift
// Exemplo de binaryTarget (checksum da v0.1.0)
.binaryTarget(
    name: "ReaderFfiFFI",
    url: "https://github.com/andreAlvesCarvalhojr/reader-engine/releases/download/v0.1.0/ReaderFfiFFI.xcframework.zip",
    checksum: "b02f22ed40f9766c2907573b5da3a8b31a00271a03d6685ab0cea37ae8d7b3c1"
)
```

2. Inclua também as fontes Swift geradas pelo UniFFI (pasta `Sources/ReaderFfi` após o build do script, ou clone este repo e aponte o pacote para `platforms/ios` com o `Package.swift` atualizado para o `url` acima).

> O `Package.swift` em `platforms/ios` usa path local por padrão (`build/ReaderFfiFFI.xcframework`). Para consumo remoto, troque o `binaryTarget` pelo bloco `url` + `checksum` acima.

Assets: [Release v0.1.0](https://github.com/andreAlvesCarvalhojr/reader-engine/releases/tag/v0.1.0).

### 3. Código no app (SwiftUI / UIKit)

```swift
import ReaderEngine
import WebKit

func openBook(webView: WKWebView, fileName: String, data: Data) throws {
    let engine = ReaderEngine()
    let session = try engine.open(name: fileName, bytes: data)
    let html = try session.currentHtml()
    webView.loadHTMLString(html, baseURL: nil)

    // session.next() / session.prev() / session.goTo(spineIndex: 0)
    // session.setFontScale(scale: 1.2)
}
```

Mais detalhes: [`platforms/ios/README.md`](platforms/ios/README.md).

---

## Como usar — Web (WASM)

### 1. Build do pacote WASM

```bash
# precisa de wasm-pack: cargo install wasm-pack
./scripts/build-web.sh
```

Gera `platforms/web/pkg/` (não versionado no git).

### 2. Sirva por HTTP (não use `file://`)

```bash
python3 -m http.server 8788 --directory platforms/web
# abra http://127.0.0.1:8788
```

### 3. Integrar no seu front

```js
import init, { WasmReaderEngine } from "./pkg/reader_wasm.js";

await init();
const engine = new WasmReaderEngine();

// bytes = Uint8Array de um .epub / .json / .hpub
const session = engine.open("livro.epub", bytes);
const snap = session.snapshot();
document.querySelector("iframe").srcdoc = session.currentHtml();

session.next();
session.setTheme("material3"); // material3 | hig | fluent
session.setFontScale(1.2);
```

A UI pronta em `platforms/web` já inclui amostras, picker de arquivo, sumário, busca e bookmarks.

Mais detalhes: [`platforms/web/README.md`](platforms/web/README.md).

---

## Como usar — Demo Rust (servidor local)

```bash
cargo run -p reader-demo
# http://127.0.0.1:8787
```

Útil para testar parsers/temas sem app mobile.

---

## Desenvolvimento neste repositório

Requer Rust 1.85+ (`rust-toolchain.toml`).

```bash
cargo test --workspace
cargo clippy --workspace --all-targets
```

### Rebuild Android (neste monorepo)

```bash
export ANDROID_HOME="$HOME/Library/Android/sdk"   # Mac
./scripts/android-build-rust.sh
cd platforms/android && ./gradlew :reader-engine:assembleRelease :app:assembleDebug
```

### Publicar nova versão

```bash
git tag v0.2.0
git push origin v0.2.0
```

O workflow `publish-mobile.yml` gera AAR (GitHub Packages + Release) e XCFramework (Release).

---

## Formato customizado

```rust
let mut reg = Registry::new();
reg.register(Box::new(EpubParser)).register(Box::new(MyParser));
let publication = reg.open(Source { name: "livro.xyz".into(), bytes })?;
```

## Próximos passos

1. App demo iOS (SwiftUI + WKWebView).
2. Empacotar `platforms/web` (PWA / CDN).
3. Mais fixtures e ContentHost compartilhado.

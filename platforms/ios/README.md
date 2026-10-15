# iOS (Swift Package + XCFramework)

Library UniFFI para apps iOS/SwiftUI. O app entrega `(nome, bytes)` — a engine não lê disco.

## Build local (macOS + Xcode)

Na raiz do monorepo:

```bash
./scripts/ios-build-xcframework.sh
```

Isso gera:

- `platforms/ios/build/ReaderFfiFFI.xcframework`
- `platforms/ios/build/ReaderFfiFFI.xcframework.zip` + checksum
- bindings Swift em `platforms/ios/Sources/ReaderFfi/`

Abra `platforms/ios` como Swift Package no Xcode (File → Add Package Dependencies → Add Local…).

## Uso no app

```swift
import ReaderEngine

let engine = ReaderEngine()
let session = try engine.open(name: "book.epub", bytes: Data(epubBytes))
let html = try session.currentHtml()
// carregue `html` em WKWebView
```

## Publicação via GitHub Release

1. Rode `./scripts/ios-build-xcframework.sh` no Mac (ou job `macos` no CI).
2. Anexe `ReaderFfiFFI.xcframework.zip` ao Release `vX.Y.Z`.
3. Atualize `Package.swift` para:

```swift
.binaryTarget(
    name: "ReaderFfiFFI",
    url: "https://github.com/andreAlvesCarvalhojr/reader-engine/releases/download/vX.Y.Z/ReaderFfiFFI.xcframework.zip",
    checksum: "<checksum impresso pelo script>"
)
```

Consumidores adicionam o pacote pela URL do repositório.

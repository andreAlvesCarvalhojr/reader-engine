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

## Consumo via GitHub Release (v0.1.0)

Assets: https://github.com/andreAlvesCarvalhojr/reader-engine/releases/tag/v0.1.0

Para apontar o `binaryTarget` ao zip publicado, troque o path local em `Package.swift` por:

```swift
.binaryTarget(
    name: "ReaderFfiFFI",
    url: "https://github.com/andreAlvesCarvalhojr/reader-engine/releases/download/v0.1.0/ReaderFfiFFI.xcframework.zip",
    checksum: "b02f22ed40f9766c2907573b5da3a8b31a00271a03d6685ab0cea37ae8d7b3c1"
)
```

Em releases novas, rode `./scripts/ios-build-xcframework.sh` (ou use o asset/checksum do Release) e atualize `url` + `checksum`.

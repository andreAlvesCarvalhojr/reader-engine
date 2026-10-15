# Android

Dois módulos Gradle:

| Módulo | Papel |
| --- | --- |
| `:reader-engine` | **Library (AAR)** — UniFFI Kotlin + `libreader_ffi.so` + JNA |
| `:app` | Demo Compose + WebView (depende da library) |

A engine **não lê disco**: o app obtém bytes e chama `ReaderEngine.open(name, bytes)`.

## Consumir a library em outro projeto

### Opção A — GitHub Packages (após tag `vX.Y.Z`)

```kotlin
// settings.gradle.kts
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

// app/build.gradle.kts
dependencies {
    implementation("com.github.andreAlvesCarvalhojr:reader-engine:0.1.0")
}
```

Crie um PAT com `read:packages` (ou use `GITHUB_TOKEN` em Actions).

### Opção B — AAR do GitHub Release

Baixe [`reader-engine-release.aar`](https://github.com/andreAlvesCarvalhojr/reader-engine/releases/download/v0.1.0/reader-engine-release.aar) (Release [v0.1.0](https://github.com/andreAlvesCarvalhojr/reader-engine/releases/tag/v0.1.0)) e:

```kotlin
dependencies {
    implementation(files("libs/reader-engine-release.aar"))
    implementation("net.java.dev.jna:jna:5.15.0@aar")
}
```

### Uso

```kotlin
import uniffi.reader_ffi.ReaderEngine

val session = ReaderEngine().open(fileName, bytes)
webView.loadDataWithBaseURL(null, session.currentHtml(), "text/html", "utf-8", null)
```

## Desenvolvimento neste monorepo

### Pré-requisitos

- Android Studio / JDK 17+
- Rust (`rustup`)
- Android NDK
- `ANDROID_HOME` (o script acha o NDK sozinho)

### Build da lib nativa

Na **raiz do monorepo**:

```bash
export ANDROID_HOME="$HOME/Library/Android/sdk"   # Mac
./scripts/android-build-rust.sh
```

Gera:

- `reader-engine/src/main/jniLibs/{arm64-v8a,armeabi-v7a,x86_64}/libreader_ffi.so`
- bindings em `reader-engine/src/main/java/uniffi/reader_ffi/`

### Demo app + AAR local

```bash
cd platforms/android
./gradlew :reader-engine:assembleRelease :app:assembleDebug
```

### Amostras locais (gitignored)

```bash
python3 scripts/convert-lessons-pt.py ~/Desktop/lesson-json/arquivo.json
./scripts/install-local-sample.sh
```

## Publicar

Tag `v0.1.0` dispara `.github/workflows/publish-mobile.yml`:

1. Build NDK + AAR
2. Publish no GitHub Packages
3. Build XCFramework (macOS) e anexa AAR + zip iOS no Release

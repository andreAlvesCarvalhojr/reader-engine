# Android

Dois módulos Gradle:

| Módulo | Papel |
| --- | --- |
| `:reader-engine` | **Library (AAR)** — UniFFI Kotlin + `libreader_ffi.so` + JNA |
| `:app` | Demo Compose + WebView (depende da library) |

A engine **não lê disco**: o app obtém bytes e chama `ReaderEngine.open(name, bytes)`.

## Consumir em outro projeto (sem token)

### 1. Repositório Maven público

```kotlin
// settings.gradle.kts
dependencyResolutionManagement {
    repositories {
        google()
        mavenCentral()
        maven {
            url = uri("https://reader-engine-maven.vercel.app/")
        }
    }
}
```

### 2. Dependência

```kotlin
// app/build.gradle.kts
dependencies {
    implementation("com.github.andreAlvesCarvalhojr:reader-engine:0.1.0")
}
```

Não precisa de `GITHUB_TOKEN`, PAT nem `GITHUB_ACTOR`. Sync Gradle e use a API.

### Alternativa — AAR local

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

### Publicar layout Maven (consumo sem token)

```bash
VERSION_NAME=0.1.0 ./scripts/publish-public-maven.sh
# Opcional: PUSH_MVN_REPO=1 VERSION_NAME=0.1.0 ./scripts/publish-public-maven.sh
# Deploy CDN: ./scripts/deploy-public-maven-vercel.sh
```

## Publicar release

Tag `v0.1.0` dispara `.github/workflows/publish-mobile.yml`:

1. Build NDK + AAR
2. Publica na branch **`mvn-repo`** e no host público Maven
3. Anexa AAR + XCFramework no GitHub Release

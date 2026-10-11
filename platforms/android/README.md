# Android (Compose + WebView)

O app **não lê disco na engine**: ele obtém os bytes (asset de amostra ou Storage Access Framework) e chama:

```kotlin
val session = ReaderEngine().open(fileName, bytes)
webView.loadDataWithBaseURL(null, session.currentHtml(), "text/html", "utf-8", null)
```

## Fluxos

| Fluxo | Quando usar |
| --- | --- |
| Assets (`samples/*.json`, `samples/*.epub`) | Demo / testes sem picker |
| **Abrir arquivo** | Uso real: usuário escolhe `.epub` / `.json` |

## Pré-requisitos

- Android Studio Ladybug+ (JDK 17)
- Rust (`rustup`)
- Android NDK (via SDK Manager)
- `ANDROID_NDK_HOME` ou `ANDROID_HOME` com NDK instalado

## Build da lib nativa

Na raiz do monorepo:

```bash
export ANDROID_NDK_HOME=$ANDROID_HOME/ndk/<versão>
./scripts/android-build-rust.sh
```

Isso gera:

- `app/src/main/jniLibs/{arm64-v8a,armeabi-v7a,x86_64}/libreader_ffi.so`
- bindings Kotlin em `app/src/main/java/uniffi/reader_ffi/`

## Rodar o app

```bash
cd platforms/android
# Abra a pasta no Android Studio, sync Gradle e Run.
# Ou, com wrapper gerado pelo Studio:
./gradlew :app:assembleDebug
```

As bindings Kotlin já vêm commitadas; as `.so` precisam ser geradas localmente (não versionamos binários grandes).

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
| Assets locais (`samples/local/*`, gitignored) | Livros grandes de teste (ex.: `lessons-pt.json`) |
| **Abrir arquivo** | Uso real: usuário escolhe `.epub` / `.json` |

### Amostra local `lessons-pt.json` (não vai pro git)

O arquivo bruto de lições (array JSON) precisa ser convertido para `reader-json`.
Use o **caminho real** do arquivo no Mac (não um placeholder):

```bash
# Veja o nome do arquivo dentro da pasta:
ls ~/Desktop/lesson-json

# Converta o .json (arquivo ou pasta que o contém):
python3 scripts/convert-lessons-pt.py ~/Desktop/lesson-json/NOME-DO-ARQUIVO.json
# ou:
python3 scripts/convert-lessons-pt.py ~/Desktop/lesson-json

./scripts/install-local-sample.sh
cd platforms/android && ./gradlew :app:installDebug
```

Dica: no Terminal, arraste o **arquivo** `.json` (não a pasta) do Finder para a janela.

Depois disso aparece o chip **lessons-pt** no app. Sem o arquivo em `assets/samples/local/`, só ficam Noite Clara e EPUB Fixture.

## Pré-requisitos

- Android Studio com AGP 9+ (JDK 17+; Gradle 9.1 roda em JVM até 25)
- Rust (`rustup`)
- Android NDK (via SDK Manager)
- `ANDROID_NDK_HOME` ou `ANDROID_HOME` com NDK instalado
- SDK Platform 36 + Build-Tools 36.0.0 (Android Studio → SDK Manager)

## Build da lib nativa

Na **raiz do monorepo** (não dentro de `platforms/android`):

```bash
# Se ANDROID_HOME já aponta para o SDK, o script acha o NDK sozinho:
export ANDROID_HOME="$HOME/Library/Android/sdk"   # caminho típico no Mac
./scripts/android-build-rust.sh
```

Para forçar uma versão específica do NDK (substitua pelo número real de `ls $ANDROID_HOME/ndk`):

```bash
ls "$ANDROID_HOME/ndk"
export ANDROID_NDK_HOME="$ANDROID_HOME/ndk/27.0.12077973"
./scripts/android-build-rust.sh
```

Não use literais como `<versão>` no terminal — o zsh interpreta `<` e dá `parse error`.

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

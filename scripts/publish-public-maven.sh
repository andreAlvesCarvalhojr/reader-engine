#!/usr/bin/env bash
# Publica o AAR no layout Maven local (platforms/android/public-maven)
# e, opcionalmente, faz push para a branch mvn-repo (consumo sem token).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
VERSION="${VERSION_NAME:-${1:-}}"
if [[ -z "${VERSION}" ]]; then
  echo "Uso: VERSION_NAME=0.1.0 $0" >&2
  echo "  ou: $0 0.1.0" >&2
  exit 1
fi
export VERSION_NAME="${VERSION}"
if [[ -z "${ANDROID_HOME:-}" ]]; then
  if [[ -d "$HOME/Android/Sdk" ]]; then
    export ANDROID_HOME="$HOME/Android/Sdk"
  elif [[ -d "$HOME/Library/Android/sdk" ]]; then
    export ANDROID_HOME="$HOME/Library/Android/sdk"
  fi
fi

cd "$ROOT"
./scripts/android-build-rust.sh
(
  cd platforms/android
  ./gradlew :reader-engine:publishReleasePublicationToPublicMavenRepository --no-daemon
)

echo "OK: platforms/android/public-maven"
echo "Consumo (sem credenciais):"
echo "  maven { url = uri(\"https://reader-engine-maven.vercel.app/\") }"
echo "  implementation(\"com.github.andreAlvesCarvalhojr:reader-engine:${VERSION}\")"

if [[ "${PUSH_MVN_REPO:-}" == "1" ]]; then
  TMP="$(mktemp -d)"
  trap 'rm -rf "$TMP"' EXIT
  git clone --branch mvn-repo --single-branch \
    "https://github.com/andreAlvesCarvalhojr/reader-engine.git" "$TMP" \
    || {
      git clone "https://github.com/andreAlvesCarvalhojr/reader-engine.git" "$TMP"
      (
        cd "$TMP"
        git checkout --orphan mvn-repo
        git rm -rf . >/dev/null 2>&1 || true
      )
    }
  (
    cd "$TMP"
    git checkout mvn-repo 2>/dev/null || true
    # Merge new version into existing tree (keep prior versions)
    cp -a "$ROOT/platforms/android/public-maven/." .
    git add -A
    git commit -m "maven: publish reader-engine ${VERSION}" || true
    git push -u origin mvn-repo
  )
fi

if [[ "${DEPLOY_VERCEL:-}" == "1" ]]; then
  "$ROOT/scripts/deploy-public-maven-vercel.sh"
fi

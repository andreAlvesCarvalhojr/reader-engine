#!/usr/bin/env bash
# Deploy platforms/android/public-maven to the public Maven host (no consumer auth).
# Requires: Vercel CLI auth, or VERCEL_TOKEN env.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC="${ROOT}/platforms/android/public-maven"
if [[ ! -d "${SRC}/com" ]]; then
  echo "Missing ${SRC} — run VERSION_NAME=x.y.z ./scripts/publish-public-maven.sh first" >&2
  exit 1
fi

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
cp -a "${SRC}/." "$TMP/"
cat > "$TMP/vercel.json" <<'EOF'
{
  "headers": [
    {
      "source": "/(.*)",
      "headers": [
        { "key": "Access-Control-Allow-Origin", "value": "*" },
        { "key": "Access-Control-Allow-Methods", "value": "GET, HEAD, OPTIONS" }
      ]
    },
    {
      "source": "/(.*)\\.aar",
      "headers": [{ "key": "Content-Type", "value": "application/java-archive" }]
    },
    {
      "source": "/(.*)\\.jar",
      "headers": [{ "key": "Content-Type", "value": "application/java-archive" }]
    },
    {
      "source": "/(.*)\\.pom",
      "headers": [{ "key": "Content-Type", "value": "application/xml" }]
    },
    {
      "source": "/(.*)\\.module",
      "headers": [{ "key": "Content-Type", "value": "application/json" }]
    },
    {
      "source": "/(.*)\\.xml",
      "headers": [{ "key": "Content-Type", "value": "application/xml" }]
    }
  ]
}
EOF
cat > "$TMP/index.html" <<'EOF'
<!doctype html><meta charset=utf-8><title>reader-engine Maven</title>
<pre>Public Maven repository for reader-engine
maven { url = uri("https://reader-engine-maven.vercel.app/") }
implementation("com.github.andreAlvesCarvalhojr:reader-engine:VERSION")
</pre>
EOF

cd "$TMP"
ARGS=(deploy --yes --prod --name reader-engine-maven)
if [[ -n "${VERCEL_TOKEN:-}" ]]; then
  ARGS+=(--token "$VERCEL_TOKEN")
fi
npx -y vercel@latest "${ARGS[@]}"

echo "OK: https://reader-engine-maven.vercel.app/"
echo "  implementation(\"com.github.andreAlvesCarvalhojr:reader-engine:${VERSION_NAME:-VERSION}\")"

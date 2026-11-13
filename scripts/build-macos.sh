#!/usr/bin/env bash
# Version macOS signée Developer ID et notarisée, d'après src-tauri/keys/apple.local.
# Sans ce fichier : signature ad hoc (l'autorisation d'accès aux dossiers est redemandée à chaque mise à jour).
set -euo pipefail
cd "$(dirname "$0")/.."

CLES=src-tauri/keys/apple.local
if [[ -f "$CLES" ]]; then
  set -a; source "$CLES"; set +a
  if ! security find-identity -v -p codesigning | grep -qF "$APPLE_SIGNING_IDENTITY"; then
    echo "Identité absente du trousseau : $APPLE_SIGNING_IDENTITY" >&2
    exit 1
  fi
else
  echo "⚠️  $CLES absent : signature ad hoc, sans notarisation."
fi

npx tauri build "$@"

APP=src-tauri/target/release/bundle/macos/RustMusic.app
codesign --verify --deep --strict --verbose=2 "$APP"
codesign -dv "$APP" 2>&1 | grep -E "Identifier=|Authority=|TeamIdentifier=" || true
if [[ -f "$CLES" ]]; then
  spctl -a -vv "$APP"
  xcrun stapler validate "$APP"
fi

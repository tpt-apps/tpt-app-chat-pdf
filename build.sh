#!/usr/bin/env sh
# TPT Chat PDF - build pipeline (macOS/Linux). Windows canonical: build.ps1.
#
# Free edition (default):  ./build.sh
#   cargo build --release --target wasm32-unknown-unknown, then
#   wasm-bindgen --target web -> $TPT_WEB_REPO/public/apps/chat-pdf/ (glue
#   named chat-pdf.js, wasm keeps its _bg.wasm name), otherwise dist/web.
# Pro desktop edition:     ./build.sh --pro  (Phase 3 - not yet implemented)
#
# Env:
#   TPT_WEB_REPO  path to the web repo; the bundle is copied to
#                 $TPT_WEB_REPO/public/apps/chat-pdf/, otherwise dist/web.

set -eu
cd "$(dirname "$0")"

CRATE=tpt-app-chat-pdf
TARGET=wasm32-unknown-unknown
OUT_NAME=chat-pdf

for arg in "$@"; do
  case "$arg" in
    --pro) echo "The Pro edition build is Windows-only for now (build.ps1 -Pro)." >&2; exit 2 ;;
    *) echo "usage: build.sh [--pro]" >&2; exit 2 ;;
  esac
done

if [ -n "${TPT_WEB_REPO:-}" ]; then
  DEST="$TPT_WEB_REPO/public/apps/chat-pdf"
else
  DEST="dist/web"
fi

echo "== TPT Chat PDF build =="
echo " edition: free (one small PDF, in-browser)"
echo " dest:    $DEST"

cargo build --release --target "$TARGET" -p "$CRATE"

WASM="target/$TARGET/release/tpt_app_chat_pdf.wasm"
[ -f "$WASM" ] || { echo "missing $WASM" >&2; exit 1; }

mkdir -p "$DEST"
wasm-bindgen --target web --out-name "$OUT_NAME" --out-dir "$DEST" "$WASM"

GLUE="$DEST/$OUT_NAME.js"
BG_WASM="$DEST/${OUT_NAME}_bg.wasm"
[ -f "$GLUE" ] || { echo "wasm-bindgen did not produce $GLUE" >&2; exit 1; }
[ -f "$BG_WASM" ] || { echo "wasm-bindgen did not produce $BG_WASM" >&2; exit 1; }

# Payload size report (local-model weights are tracked separately from this
# code-size line).
RAW=$(wc -c < "$BG_WASM" | tr -d ' ')
if command -v gzip >/dev/null 2>&1; then
  GZ=$(gzip -9 -c "$BG_WASM" | wc -c | tr -d ' ')
  echo "-- $(basename "$BG_WASM"): $RAW bytes raw, $GZ bytes gzipped"
fi

echo "-- done"
[ -n "${TPT_WEB_REPO:-}" ] || echo "   (set TPT_WEB_REPO to copy the bundle into the hub web repo)"

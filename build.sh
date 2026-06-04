#!/usr/bin/env bash
# Render build script: downloads CJK font + builds Rust binary
set -euo pipefail

echo "==> Downloading Noto Sans SC (CJK font for Chinese support)..."
mkdir -p fonts

FONT_URLS=(
  "https://github.com/notofonts/noto-cjk/raw/main/Sans/OTF/SimplifiedChinese/NotoSansSC-Regular.otf"
  "https://github.com/googlefonts/noto-cjk/raw/main/Sans/OTF/SimplifiedChinese/NotoSansSC-Regular.otf"
)

DOWNLOADED=false
for url in "${FONT_URLS[@]}"; do
  echo "    Trying: $url"
  if curl -fsSL -o fonts/NotoSansSC-Regular.otf "$url" 2>/dev/null; then
    SIZE=$(stat -f%z fonts/NotoSansSC-Regular.otf 2>/dev/null || stat -c%s fonts/NotoSansSC-Regular.otf 2>/dev/null || echo "0")
    if [ "$SIZE" -gt 1000 ]; then
      echo "    OK (${SIZE} bytes)"
      DOWNLOADED=true
      break
    fi
  fi
  echo "    Failed, trying next..."
done

if [ "$DOWNLOADED" = false ]; then
  echo "WARNING: Could not download CJK font. Chinese text may not render correctly."
fi

echo "==> Building svg2png-api..."
cargo build --release

echo "==> Build complete."

#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -lt 1 ] || [ "$#" -gt 2 ]; then
  echo "Usage: $0 <input-image> [output-png]" >&2
  exit 1
fi

INPUT="$1"
if [ "$#" -eq 2 ]; then
  OUTPUT="$2"
else
  BASE="${INPUT%.*}"
  OUTPUT="${BASE}-edited.png"
fi

exec flatpak run --command=photocraft-cli ai.storyteller.photocraft run \
  "$INPUT" \
  --cmd image.imageSize \
  --params '{"percent":400,"resample":"bicubicSmoother"}' \
  --cmd filter.noise.addNoise \
  --params '{"amount":10,"distribution":"gaussian","monochromatic":true}' \
  --out "$OUTPUT"

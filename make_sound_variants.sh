#!/usr/bin/env bash
set -euo pipefail
shopt -s nullglob

SRC="assets/sounds/src"
DEST="assets/sounds/dest"
mkdir -p "$DEST"

# 9 pitch multipliers (roughly: -4, -3, -2, -1, 0, +1, +2, +3, +4 semitones)
PITCH=(0.793701 0.840896 0.890899 0.943874 1.000000 1.059463 1.122462 1.189207 1.259921)

# small volume jitters (dB)
GAIN=(-1.5 -1.0 -0.5 -0.2 0.0 0.2 0.5 1.0 1.5)

i=0
for f in "$SRC"/*.wav; do
  base="$(basename "$f" .wav)"
  for v in {1..9}; do
    p="${PITCH[$((v-1))]}"
    g="${GAIN[$((v-1))]}"
    out="$DEST/${base}__v$(printf "%02d" "$v").wav"

    ffmpeg -hide_banner -loglevel error -y \
      -i "$f" \
      -af "rubberband=pitch=${p},volume=${g}dB" \
      -c:a pcm_s16le \
      "$out"
  done
  echo "Made 9 variants for $base.wav"
done


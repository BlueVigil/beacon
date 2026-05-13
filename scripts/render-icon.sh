#!/usr/bin/env bash
set -euo pipefail

size="${1:-1024}"
shader="src/icon/beacon.glsl"
out_dir="target/icon"
png="${out_dir}/beacon-${size}.png"
ico="${out_dir}/beacon.ico"
render_shader="${out_dir}/beacon.frag"

mkdir -p "${out_dir}"

if ! command -v glslViewer >/dev/null 2>&1; then
  printf 'missing glslViewer. install it first, for example: brew install glslviewer\n' >&2
  exit 1
fi

cp "${shader}" "${render_shader}"
glslViewer "${render_shader}" --headless --noncurses -w "${size}" -h "${size}" -E screenshot,"${png}"
printf 'wrote %s\n' "${png}"

if command -v magick >/dev/null 2>&1; then
  magick "${png}" -define icon:auto-resize=256,128,64,48,32,16 "${ico}"
  printf 'wrote %s\n' "${ico}"
elif command -v convert >/dev/null 2>&1; then
  convert "${png}" -define icon:auto-resize=256,128,64,48,32,16 "${ico}"
  printf 'wrote %s\n' "${ico}"
else
  printf 'skipped ico export: install ImageMagick for magick/convert\n' >&2
fi

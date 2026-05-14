#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only

set -euo pipefail

size="${1:-1024}"
shader="src/icon/beacon.glsl"
out_dir="target/icon"
png="${out_dir}/beacon-${size}.png"
ico="${out_dir}/beacon.ico"
render_shader="${out_dir}/beacon.frag"
asset_dir="assets/icons"
asset_png="${asset_dir}/beacon.png"
asset_ico="${asset_dir}/beacon.ico"
asset_icns="${asset_dir}/Beacon.icns"

mkdir -p "${out_dir}"
mkdir -p "${asset_dir}"

if ! command -v glslViewer >/dev/null 2>&1; then
  printf 'missing glslViewer. install it first, for example: brew install glslviewer\n' >&2
  exit 1
fi

cp "${shader}" "${render_shader}"
render_log="${out_dir}/render.log"
if ! glslViewer "${render_shader}" --headless --noncurses -w "${size}" -h "${size}" -E screenshot,"${png}" >"${render_log}" 2>&1; then
  cat "${render_log}" >&2
  exit 1
fi

if grep -q "Found error while compiling" "${render_log}"; then
  cat "${render_log}" >&2
  exit 1
fi

printf 'wrote %s\n' "${png}"
cp "${png}" "${asset_png}"
printf 'wrote %s\n' "${asset_png}"

if command -v magick >/dev/null 2>&1; then
   magick "${png}" -define icon:auto-resize=256,128,64,48,32,16 "${ico}"
   printf 'wrote %s\n' "${ico}"
  cp "${ico}" "${asset_ico}"
  printf 'wrote %s\n' "${asset_ico}"
elif command -v convert >/dev/null 2>&1; then
   convert "${png}" -define icon:auto-resize=256,128,64,48,32,16 "${ico}"
   printf 'wrote %s\n' "${ico}"
  cp "${ico}" "${asset_ico}"
  printf 'wrote %s\n' "${asset_ico}"
else
   printf 'skipped ico export: install ImageMagick for magick/convert\n' >&2
fi

if command -v sips >/dev/null 2>&1 && command -v iconutil >/dev/null 2>&1; then
  iconset="${out_dir}/Beacon.iconset"
  rm -rf "${iconset}"
  mkdir -p "${iconset}"
  sips -z 16 16 "${png}" --out "${iconset}/icon_16x16.png" >/dev/null
  sips -z 32 32 "${png}" --out "${iconset}/icon_16x16@2x.png" >/dev/null
  sips -z 32 32 "${png}" --out "${iconset}/icon_32x32.png" >/dev/null
  sips -z 64 64 "${png}" --out "${iconset}/icon_32x32@2x.png" >/dev/null
  sips -z 128 128 "${png}" --out "${iconset}/icon_128x128.png" >/dev/null
  sips -z 256 256 "${png}" --out "${iconset}/icon_128x128@2x.png" >/dev/null
  sips -z 256 256 "${png}" --out "${iconset}/icon_256x256.png" >/dev/null
  sips -z 512 512 "${png}" --out "${iconset}/icon_256x256@2x.png" >/dev/null
  sips -z 512 512 "${png}" --out "${iconset}/icon_512x512.png" >/dev/null
  sips -z 1024 1024 "${png}" --out "${iconset}/icon_512x512@2x.png" >/dev/null
  iconutil -c icns "${iconset}" -o "${asset_icns}"
  printf 'wrote %s\n' "${asset_icns}"
else
  printf 'skipped icns export: sips and iconutil are required\n' >&2
fi

#!/usr/bin/env bash
set -euo pipefail

target="${1:?usage: scripts/package.sh <target-triple>}"
profile="${PROFILE:-release}"
package_root="target/packages/${target}"
binary_name="beacon"
exe_name="beacon"

case "${target}" in
  *windows*)
    platform="windows"
    exe_name="beacon.exe"
    tycmd_name="tycmd.exe"
    ;;
  *apple-darwin*)
    platform="macos"
    tycmd_name="tycmd"
    ;;
  *linux*)
    platform="linux"
    tycmd_name="tycmd"
    ;;
  *)
    printf 'unsupported target: %s\n' "${target}" >&2
    exit 1
    ;;
esac

case "${target}" in
  aarch64-*|arm64-*) arch="aarch64" ;;
  x86_64-*) arch="x86_64" ;;
  i686-*) arch="i686" ;;
  *)
    printf 'unsupported target architecture: %s\n' "${target}" >&2
    exit 1
    ;;
esac

RUSTC="$(rustup which --toolchain nightly rustc)" rustup run nightly cargo build --release --target "${target}"

rm -rf "${package_root}"
mkdir -p "${package_root}"

binary_path="target/${target}/${profile}/${exe_name}"
if [[ ! -f "${binary_path}" ]]; then
  printf 'built binary missing: %s\n' "${binary_path}" >&2
  exit 1
fi

copy_resources() {
  local destination="$1"
  mkdir -p "${destination}"
  cp -R assets "${destination}/assets"
  mkdir -p "${destination}/tycmd/${platform}"

  if [[ -d "resources/tycmd/${platform}/${arch}" ]]; then
    mkdir -p "${destination}/tycmd/${platform}/${arch}"
    cp -R "resources/tycmd/${platform}/${arch}/." "${destination}/tycmd/${platform}/${arch}/"
  fi

  if [[ -f "resources/tycmd/${platform}/${tycmd_name}" ]]; then
    cp "resources/tycmd/${platform}/${tycmd_name}" "${destination}/tycmd/${platform}/${tycmd_name}"
  fi

  if [[ "${platform}" != "windows" ]]; then
    chmod -R u+rwX "${destination}/tycmd" || true
    find "${destination}/tycmd" -type f -name "tycmd" -exec chmod +x {} \;
  fi
}

if [[ "${platform}" == "macos" ]]; then
  app_dir="${package_root}/Beacon.app"
  mkdir -p "${app_dir}/Contents/MacOS" "${app_dir}/Contents/Resources"
  cp "${binary_path}" "${app_dir}/Contents/MacOS/${binary_name}"
  chmod +x "${app_dir}/Contents/MacOS/${binary_name}"
  copy_resources "${app_dir}/Contents/Resources"
  if [[ -f "assets/icons/Beacon.icns" ]]; then
    cp "assets/icons/Beacon.icns" "${app_dir}/Contents/Resources/Beacon.icns"
  fi
  cat > "${app_dir}/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "https://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
  <dict>
    <key>CFBundleExecutable</key>
    <string>beacon</string>
    <key>CFBundleIdentifier</key>
    <string>dev.beacon.app</string>
    <key>CFBundleName</key>
    <string>Beacon</string>
    <key>CFBundleIconFile</key>
    <string>Beacon</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleVersion</key>
    <string>0.1.0</string>
    <key>CFBundleShortVersionString</key>
    <string>0.1.0</string>
    <key>LSMinimumSystemVersion</key>
    <string>14.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
  </dict>
</plist>
PLIST
else
  app_dir="${package_root}/beacon"
  mkdir -p "${app_dir}"
  cp "${binary_path}" "${app_dir}/${exe_name}"
  [[ "${platform}" == "windows" ]] || chmod +x "${app_dir}/${exe_name}"
  copy_resources "${app_dir}/resources"
fi

printf 'packaged %s at %s\n' "${target}" "${app_dir}"

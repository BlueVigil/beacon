#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only

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
    ;;
  *apple-darwin*)
    platform="macos"
    ;;
  *linux*)
    platform="linux"
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
}

if [[ "${platform}" == "macos" ]]; then
  app_dir="${package_root}/BEACON.app"
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
    <string>BEACON</string>
    <key>CFBundleDisplayName</key>
    <string>BEACON</string>
    <key>CFBundleIconFile</key>
    <string>Beacon</string>
    <key>CFBundleDocumentTypes</key>
    <array>
      <dict>
        <key>CFBundleTypeName</key>
        <string>Intel HEX Firmware</string>
        <key>CFBundleTypeRole</key>
        <string>Viewer</string>
        <key>CFBundleTypeExtensions</key>
        <array>
          <string>hex</string>
        </array>
        <key>LSHandlerRank</key>
        <string>Alternate</string>
      </dict>
    </array>
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
  if [[ "${platform}" == "linux" ]]; then
    mkdir -p "${app_dir}/share/applications" "${app_dir}/share/mime/packages" "${app_dir}/share/icons/hicolor/1024x1024/apps"
    if [[ -f "assets/icons/beacon.png" ]]; then
      cp "assets/icons/beacon.png" "${app_dir}/share/icons/hicolor/1024x1024/apps/dev.beacon.BEACON.png"
    fi
    cat > "${app_dir}/share/applications/dev.beacon.BEACON.desktop" <<'DESKTOP'
[Desktop Entry]
Type=Application
Name=BEACON
Comment=Flash Teensy firmware
Exec=beacon %f
Icon=dev.beacon.BEACON
Terminal=false
Categories=Development;Electronics;
MimeType=application/x-intel-hex;text/x-hex;
DESKTOP
    cat > "${app_dir}/share/mime/packages/dev.beacon.BEACON.xml" <<'MIME'
<?xml version="1.0" encoding="UTF-8"?>
<mime-info xmlns="http://www.freedesktop.org/standards/shared-mime-info">
  <mime-type type="application/x-intel-hex">
    <comment>Intel HEX firmware</comment>
    <glob pattern="*.hex"/>
  </mime-type>
</mime-info>
MIME
  elif [[ "${platform}" == "windows" ]]; then
    cat > "${app_dir}/install-file-association.reg" <<'REG'
Windows Registry Editor Version 5.00

[HKEY_CURRENT_USER\Software\Classes\.hex]
@="BEACON.hex"

[HKEY_CURRENT_USER\Software\Classes\BEACON.hex]
@="Intel HEX Firmware"

[HKEY_CURRENT_USER\Software\Classes\BEACON.hex\DefaultIcon]
@="\"%LOCALAPPDATA%\\BEACON\\beacon.exe\",0"

[HKEY_CURRENT_USER\Software\Classes\BEACON.hex\shell\open\command]
@="\"%LOCALAPPDATA%\\BEACON\\beacon.exe\" \"%1\""
REG
  fi
fi

printf 'packaged %s at %s\n' "${target}" "${app_dir}"

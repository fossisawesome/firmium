# Maintainer: fossisawesome <lx bax wp 73 AT moz mail DOT com>
pkgname=firmium-desktop-bin
pkgver=5.3.0
pkgrel=1
pkgdesc="Lightning fast OpenSubsonic player in Tauri"
arch=('x86_64')
url="https://github.com/fossisawesome/firmium"
license=('GPL-3.0-only')
depends=('webkit2gtk-4.1' 'alsa-lib' 'openssl')
provides=('firmium-desktop')
conflicts=('firmium-desktop-git')
options=('!strip')

source=()
sha256sums=()

package() {
  # Find Tauri's deb bundle directory (handles both Firmium_* and firmium-desktop_* naming)
  local tauri_bundle_dir=$(find "${startdir}/src-tauri/target/release/bundle/deb" -maxdepth 1 -type d -name "*_${pkgver}_amd64" | head -n 1)

  if [ -z "$tauri_bundle_dir" ] || [ ! -d "$tauri_bundle_dir" ]; then
    error "Tauri Linux bundle directory not found. Ensure 'npm run tauri build' completed successfully."
    return 1
  fi

  msg2 "Extracting application assets out of Tauri's build directory..."

  # 1. Install compiled binary (handles both possible names Tauri might use)
  local binary_path
  if [ -f "${tauri_bundle_dir}/data/usr/bin/firmium-desktop" ]; then
    binary_path="${tauri_bundle_dir}/data/usr/bin/firmium-desktop"
  elif [ -f "${tauri_bundle_dir}/data/usr/bin/Firmium" ]; then
    binary_path="${tauri_bundle_dir}/data/usr/bin/Firmium"
  fi

  if [ -z "$binary_path" ]; then
    error "Compiled binary not found in bundle directory"
    return 1
  fi
  install -Dm755 "$binary_path" "${pkgdir}/usr/bin/firmium-desktop"

  # 2. Install desktop launcher entry (handles both possible names)
  local desktop_file
  for candidate in "firmium.desktop" "Firmium.desktop" "firmium-desktop.desktop"; do
    if [ -f "${tauri_bundle_dir}/data/usr/share/applications/$candidate" ]; then
      desktop_file="${tauri_bundle_dir}/data/usr/share/applications/$candidate"
      break
    fi
  done

  if [ -n "$desktop_file" ]; then
    install -Dm644 "$desktop_file" "${pkgdir}/usr/share/applications/firmium.desktop"
  fi

  # 3. Pull all processed application graphics completely over
  if [ -d "${tauri_bundle_dir}/data/usr/share/icons" ]; then
    cp -r "${tauri_bundle_dir}/data/usr/share/icons" "${pkgdir}/usr/share/"
  fi
}

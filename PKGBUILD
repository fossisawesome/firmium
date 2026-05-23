# Maintainer: fossisawesome <fossisawesome@github.com>
pkgname=firmium-desktop
pkgver=1.2.0
pkgrel=1
pkgdesc="Lightning fast OpenSubsonic player in Tauri"
arch=('x86_64')
url="https://github.com/fossisawesome/firmium"
license=('GPL3')
depends=('webkit2gtk-4.1' 'alsa-lib' 'openssl')
makedepends=('cargo' 'npm')
options=('!strip')

source=()
sha256sums=()

package() {
  # This is the exact staging folder Tauri compiles when building a Linux deb bundle
  local tauri_bundle_dir="${startdir}/src-tauri/target/release/bundle/deb/firmium-desktop_${pkgver}_amd64"

  # Fallback check in case Tauri alters the directory suffix naming conventions
  if [ ! -d "$tauri_bundle_dir" ]; then
    tauri_bundle_dir=$(find "${startdir}/src-tauri/target/release/bundle/deb" -maxdepth 1 -type d -name "firmium-desktop*" | head -n 1)
  fi

  if [ -z "$tauri_bundle_dir" ] || [ ! -d "$tauri_bundle_dir" ]; then
    error "Tauri Linux bundle directory not found. Ensure tauri build successfully completed."
    return 1
  fi

  msg2 "Extracting application assets out of Tauri's build directory..."

  # 1. Install your compiled binary executable straight into /usr/bin/
  install -Dm755 "${tauri_bundle_dir}/data/usr/bin/firmium-desktop" "${pkgdir}/usr/bin/firmium-desktop"

  # 2. Extract your custom desktop launcher entry based on your exact tauri.conf.json mapping
  if [ -f "${tauri_bundle_dir}/data/usr/share/applications/firmium.desktop" ]; then
    install -Dm644 "${tauri_bundle_dir}/data/usr/share/applications/firmium.desktop" "${pkgdir}/usr/share/applications/firmium.desktop"
  elif [ -f "${tauri_bundle_dir}/data/usr/share/applications/firmium-desktop.desktop" ]; then
    install -Dm644 "${tauri_bundle_dir}/data/usr/share/applications/firmium-desktop.desktop" "${pkgdir}/usr/share/applications/firmium.desktop"
  fi

  # 3. Pull all processed application graphics completely over
  if [ -d "${tauri_bundle_dir}/data/usr/share/icons" ]; then
    cp -r "${tauri_bundle_dir}/data/usr/share/icons" "${pkgdir}/usr/share/"
  fi
}
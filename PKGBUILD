# Maintainer: fossisawesome <fossisawesome@github.com>
pkgname=firmium-desktop
pkgver=1.3.0
pkgrel=1
pkgdesc="Lightning fast OpenSubsonic player in Tauri"
arch=('x86_64')
url="https://github.com/fossisawesome/firmium"
license=('GPL-3.0-only')
depends=('webkit2gtk-4.1' 'alsa-lib' 'openssl')
makedepends=('cargo' 'npm' 'nodejs')
options=('!strip')

source=("${pkgname}-${pkgver}.tar.gz::https://github.com/fossisawesome/firmium/archive/refs/tags/v${pkgver}.tar.gz")
sha256sums=('1fb4b0ef54c95d4430072710576d852ffe44c5048f2970a84178aef5178a5c9b')

build() {
  cd "firmium-${pkgver}"

  npm install
  npm run tauri build -- --bundles deb
}

package() {
  cd "firmium-${pkgver}"

  local tauri_bundle_dir="src-tauri/target/release/bundle/deb"

  # Find the actual extracted deb directory (Tauri naming convention)
  local deb_dir=$(find "$tauri_bundle_dir" -maxdepth 1 -type d -name "firmium-desktop*" | head -n 1)

  if [ -z "$deb_dir" ] || [ ! -d "$deb_dir" ]; then
    error "Tauri deb bundle directory not found"
    return 1
  fi

  # Install binary
  install -Dm755 "$deb_dir/data/usr/bin/firmium-desktop" "$pkgdir/usr/bin/firmium-desktop"

  # Install .desktop file
  install -Dm644 "$deb_dir/data/usr/share/applications/firmium.desktop" "$pkgdir/usr/share/applications/firmium.desktop"

  # Install icons
  cp -r "$deb_dir/data/usr/share/icons" "$pkgdir/usr/share/"
}

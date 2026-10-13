# Maintainer: fossisawesome <lx bax wp 73 AT moz mail DOT com>
pkgname=firmium-desktop-bin
pkgver=9.0.1
pkgrel=1
pkgdesc="Lightning fast OpenSubsonic music player (Tauri 2: Rust + Svelte)"
arch=('x86_64')
url="https://github.com/fossisawesome/firmium"
license=('GPL-3.0-only')
# cpal->alsa, tauri->webkit2gtk/gtk3, keyring->libsecret, tauri->libxdo, TLS->openssl.
depends=('alsa-lib' 'gtk3' 'webkit2gtk-4.1' 'libsecret' 'openssl' 'xdotool')
makedepends=('cargo' 'cmake' 'nodejs' 'npm')
provides=('firmium-desktop')
conflicts=('firmium-desktop-git')
options=('!strip')

# Built from the working tree (run `makepkg` from the repo root).
source=()
sha256sums=()

build() {
  cd "$startdir"
  export RUSTUP_TOOLCHAIN=stable
  npm ci
  npm run build
  cargo build --release --locked -p firmium --features custom-protocol
}

package() {
  cd "$startdir"
  install -Dm755 "target/release/firmium" "${pkgdir}/usr/bin/firmium"
  install -Dm644 "packaging/firmium.desktop" "${pkgdir}/usr/share/applications/firmium.desktop"
  install -Dm644 "src-tauri/icons/128x128.png" "${pkgdir}/usr/share/icons/hicolor/128x128/apps/firmium.png"
  install -Dm644 "src-tauri/icons/32x32.png" "${pkgdir}/usr/share/icons/hicolor/32x32/apps/firmium.png"
  install -Dm644 "LICENSE" "${pkgdir}/usr/share/licenses/${pkgname}/LICENSE"
}

# Maintainer: fossisawesome <lx bax wp 73 AT moz mail DOT com>
pkgname=firmium-desktop-bin
pkgver=8.0.0
pkgrel=1
pkgdesc="Lightning fast OpenSubsonic music player (native iced/Rust)"
arch=('x86_64')
url="https://github.com/fossisawesome/firmium"
license=('GPL-3.0-only')
# cpal->alsa, rfd->gtk3, keyring->libsecret, winit->wayland/libxkbcommon,
# wgpu->vulkan loader, iced text shaping->fontconfig.
depends=('alsa-lib' 'gtk3' 'libsecret' 'libxkbcommon' 'wayland' 'vulkan-icd-loader' 'fontconfig')
makedepends=('cargo')
provides=('firmium-desktop')
conflicts=('firmium-desktop-git')
options=('!strip')

# Built from the working tree (run `makepkg` from the repo root).
source=()
sha256sums=()

build() {
  cd "$startdir"
  export RUSTUP_TOOLCHAIN=stable
  cargo build --release --locked
}

package() {
  cd "$startdir"
  install -Dm755 "target/release/firmium" "${pkgdir}/usr/bin/firmium"
  install -Dm644 "packaging/firmium.desktop" "${pkgdir}/usr/share/applications/firmium.desktop"
  install -Dm644 "assets/app-icons/128x128.png" "${pkgdir}/usr/share/icons/hicolor/128x128/apps/firmium.png"
  install -Dm644 "assets/app-icons/32x32.png" "${pkgdir}/usr/share/icons/hicolor/32x32/apps/firmium.png"
  install -Dm644 "LICENSE" "${pkgdir}/usr/share/licenses/${pkgname}/LICENSE"
}

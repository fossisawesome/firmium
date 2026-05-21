# Maintainer: fossisawesome <fossisawesome@github.com>
pkgname=firmium-desktop
pkgver=1.0.0
pkgrel=1
pkgdesc="Lighting fast OpenSubsonic player in Tauri"
arch=('x86_64')
url="https://github.com/fossisawesome/firmium"
license=('GPL3')
depends=('webkit2gtk-4.1' 'gstreamer' 'gst-plugins-base' 'gst-plugins-good')
makedepends=('libarchive')
options=('!strip')

source=()
sha256sums=()

package() {
  bsdtar -xf "$startdir/src-tauri/target/release/bundle/deb/firmium-desktop_${pkgver}_amd64.deb" -C "$srcdir"
  
  bsdtar -xf "$srcdir/data.tar.gz" -C "${pkgdir}"
}
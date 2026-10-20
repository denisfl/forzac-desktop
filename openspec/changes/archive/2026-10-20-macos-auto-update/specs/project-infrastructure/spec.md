## MODIFIED Requirements

### Requirement: Release artefacts
A version tag `vX.Y.Z` SHALL produce release artefacts: a Linux archive for x86_64 and one for aarch64, each with the
`forzac` binary, the bundled assets under `share/forzac/`, the desktop integration files and the hicolor icons under
`share/icons/`; a Debian package for Raspberry Pi OS (arm64) built against Debian 13; a disk image with the ad-hoc
signed macOS app for Apple Silicon; the appcast `appcast.xml` for the macOS app's updates, with one item for this
version whose enclosure is this release's disk image (`releases/download/vX.Y.Z/Forzac-macos-arm64.dmg`), its length,
its EdDSA signature made with the private key held as a CI secret, and the minimum macOS version 14.0; and
`SHA256SUMS` over all of them. A tag built without the key SHALL fail rather than publish a release without an
appcast.

#### Scenario: Tagging a release
- **WHEN** a tag `v0.2.0` is pushed
- **THEN** a draft release appears with `forzac-linux-x86_64.tar.gz`, `forzac-linux-aarch64.tar.gz`,
  `forzac-linux-arm64.deb`, `Forzac-macos-arm64.dmg`, `appcast.xml` and `SHA256SUMS`

#### Scenario: The appcast verifies the image
- **WHEN** the draft's `appcast.xml` and `Forzac-macos-arm64.dmg` are downloaded
- **THEN** the item's length is the image's size and its signature verifies over the image with the public key in the
  app's `Info.plist`

#### Scenario: The Linux archive runs as unpacked
- **WHEN** a Linux archive is unpacked anywhere and `bin/forzac book.fb2` is run on a system with GTK 4
- **THEN** the book opens with the bundled fonts, without installing anything

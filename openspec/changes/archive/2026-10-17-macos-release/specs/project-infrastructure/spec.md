## MODIFIED Requirements

### Requirement: Release artefacts
A version tag `vX.Y.Z` SHALL produce release artefacts: a Linux archive for x86_64 and one for aarch64, each with the
`forzac` binary, the bundled assets under `share/forzac/`, the desktop integration files and the hicolor icons under
`share/icons/`; a Debian package for Raspberry Pi OS (arm64) built against Debian 13; and a disk image with the ad-hoc
signed macOS app for Apple Silicon, with its SHA-256 checksum.

#### Scenario: Tagging a release
- **WHEN** a tag `v0.2.0` is pushed
- **THEN** a draft release appears with `forzac-0.2.0-linux-x86_64.tar.gz`, `forzac-0.2.0-linux-aarch64.tar.gz`,
  `forzac_0.2.0_arm64.deb`, `Forzac-0.2.0-macos-arm64.dmg` and `Forzac-0.2.0-macos-arm64.dmg.sha256`

#### Scenario: The Linux archive runs as unpacked
- **WHEN** a Linux archive is unpacked anywhere and `bin/forzac book.fb2` is run on a system with GTK 4
- **THEN** the book opens with the bundled fonts, without installing anything

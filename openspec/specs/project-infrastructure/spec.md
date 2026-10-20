# project-infrastructure Specification

## Purpose
What every change to Forzac for desktop must pass before it is merged, under which licence the code is published, and
how release artefacts are produced for each supported platform.

## Requirements

### Requirement: Licence
The code SHALL be distributed under the Mozilla Public License 2.0, stated in a `LICENSE` file at the repository root
and in every crate's manifest. Bundled third-party assets SHALL keep their own licence texts next to them, and
packages SHALL install those texts.

#### Scenario: Licence is visible
- **WHEN** someone opens the repository root or a crate's metadata
- **THEN** they find the MPL-2.0 text in `LICENSE` and `MPL-2.0` as the crate licence

#### Scenario: Third-party licences travel with the assets
- **WHEN** fonts or hyphenation patterns are bundled in a package
- **THEN** their licence files are installed with them

### Requirement: Checks on every pull request
Every pull request and every push to the main branch SHALL run, and MUST fail on any error in: formatting check of all
Rust code, lints of all Rust code with warnings treated as errors, the core's unit tests, a build of the GTK shell on
Linux x86_64, a build of the GTK shell on Linux aarch64, and a build of the macOS shell on Apple Silicon.

#### Scenario: A formatting slip fails the check
- **WHEN** a pull request contains Rust code that `rustfmt` would change
- **THEN** the formatting check fails and the pull request shows a failed status

#### Scenario: Every platform is built
- **WHEN** a pull request changes the core
- **THEN** the core tests run and both shells are built on all three platforms before the pull request is marked green

### Requirement: Tests do not need private books
Tests that read real books SHALL take the books from a local directory named by the `FORZAC_CORPUS` environment
variable and SHALL be skipped, not failed, when it is unset. Books MUST NOT be committed.

#### Scenario: CI without a corpus
- **WHEN** the tests run without `FORZAC_CORPUS`
- **THEN** corpus tests are reported as skipped and the run passes

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

### Requirement: Contributor guidance
The repository SHALL document, in English, the product principles, the architecture invariants, how to build and test
each part, the OpenSpec process and the licence of contributions (MPL-2.0, no CLA).

#### Scenario: A new contributor
- **WHEN** someone reads `CONTRIBUTING.md`
- **THEN** they can build both shells, run the checks CI runs, and know that their contribution is licensed under MPL-2.0

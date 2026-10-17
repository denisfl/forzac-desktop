## 1. App bundle

- [x] 1.1 `macos/build.sh`: the icon (`macos/Forzac.icns`, `CFBundleIconFile`), ad-hoc signing of the whole bundle
      with the identifier `app.truenrth.forzac`; `codesign --verify --deep --strict` passes

## 2. Disk image

- [x] 2.1 `scripts/package-macos.sh X.Y.Z`: version in the built `Info.plist`, re-sign, DMG with the app and an
      `Applications` link (volume "Forzac"), `.sha256`
- [x] 2.2 Release workflow: the macOS job uploads the DMG and the checksum instead of the zip

## 3. First open

- [x] 3.1 `docs/macos-first-open.md`: the text for forzac.truenrth.app, macOS 14 and 15+, checksum, Terminal
      alternative
- [x] 3.2 Screenshots: moved to `release-1-0` by the maintainer (2026-10-17): screen capture is not allowed for this
      session, and macOS 15 needs another Mac; they are taken when the page is published

## 4. Checks

- [x] 4.1 On this Mac: the DMG mounts, the checksum verifies, the app copied out of it opens a book (a 760 × 875
      window) with its icon (`NSWorkspace`), `codesign --verify --deep --strict` passes and `spctl` rejects it as
      Gatekeeper rejects any ad-hoc app. The quarantined first open is part of the fresh-account check (4.2)
- [x] 4.2 Asked the maintainer: the fresh-account check, with the Gatekeeper step on a quarantined copy, is moved to
      `release-1-0`, before the DMG is published (2026-10-17)
- [x] 4.3 `cargo fmt --all --check`, clippy for the core, `cargo test -p forzac-core`; README

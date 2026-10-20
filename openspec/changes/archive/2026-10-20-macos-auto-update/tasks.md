## 1. Keys

- [x] 1.1 Generate the EdDSA key pair with Sparkle's `generate_keys --account forzac`; export the private key to
      `~/keys/forzac-sparkle-ed25519` (mode 600); note the public key
- [x] 1.2 `macos/Info.plist`: `SUFeedURL`, `SUPublicEDKey`, `SUEnableAutomaticChecks` false

## 2. Build

- [x] 2.1 `macos/build.sh`: download Sparkle 2.10.0 once into `macos/build/`, check its SHA-256, unpack to
      `macos/build/sparkle/`, link with `-F`/`-framework Sparkle` and the two rpaths, symlink `Sparkle.framework` next to
      the bare binary
- [x] 2.2 Embed the framework in `Forzac.app/Contents/Frameworks` (ditto) and Sparkle's licence in
      `Resources/licenses`; sign the app ad hoc as before; `codesign --verify --deep --strict` passes
- [x] 2.3 `scripts/package-macos.sh`: the version into `CFBundleVersion` too

## 3. The install in the library (macOS shell)

- [x] 3.1 Core catalogue: "install" / "установить", "Installing version {version}…" / "Устанавливается версия
      {version}…"
- [x] 3.2 `macos/Update.swift`: a lazily made `SPUUpdater` with user agent "Forzac" and a user driver that answers
      "install" at every step and reports failure
- [x] 3.3 `macos/Library.swift`: the line's link word "install", the "installing" and "failed" states, the click
- [x] 3.4 `cargo test -p forzac-core`, `cargo clippy -p forzac-core`, `cargo fmt --all --check` green

## 4. Release

- [x] 4.1 `scripts/appcast.sh X.Y.Z DMG`: one-item appcast signed with `sign_update` (key from
      `SPARKLE_ED_PRIVATE_KEY` or the Keychain)
- [x] 4.2 `.github/workflows/release.yml`: the macOS job makes `appcast.xml` with the secret (fails without it), the
      release job keeps it under that name and in `SHA256SUMS`
- [x] 4.3 `docs/publishing.md`: the key, the secret, the appcast, the asset table, checking the appcast before
      publishing; `README`/licences if they list bundled code

## 5. Verification on the M1 Mac

- [x] 5.1 Build an "old" app (package as 0.10.50) and a "new" one (0.10.99); sign the new DMG locally, serve it and a
      test appcast from `localhost`; point the old app at it (`defaults write app.truenrth.forzac SUFeedURL …`)
- [x] 5.2 Old app in `~/Applications` after the Terminal first-open step (replacing the reader's own `/Applications`
      copy was not allowed; the Finder route is in the release checklist); the line shows "install" (a faked `update-latest`);
      click: the line reads "Installing…", Forzac relaunches as 0.10.99 without the reader downloading anything;
      screenshots; the relaunched app opens without a Gatekeeper prompt
- [x] 5.3 Failure paths: a DMG signed with another key, the app run from the mounted DMG → "download"
- [x] 5.4 The library's first frame (`--measure`) and idle CPU unchanged; the bare `macos/build/Forzac` still runs
- [x] 5.5 Restore the reader's own Forzac and state; remove the `SUFeedURL` default

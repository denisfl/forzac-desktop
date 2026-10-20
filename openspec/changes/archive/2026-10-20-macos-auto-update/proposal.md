## Why

Since 0.10.0 the library tells the reader that a newer Forzac is out, and on the Mac the reader then downloads the disk
image again, drags the app into Applications and goes through nothing else only because the old copy was approved
already. The maintainer asked (2026-10-20) whether the app could download and install the update itself. On macOS it
can: the app is a bundle the reader owns, and Sparkle 2 updates such bundles, checking an EdDSA signature over the
update instead of an Apple Developer ID, which Forzac does not have.

## What Changes

- The macOS app embeds Sparkle 2 (`Contents/Frameworks/Sparkle.framework`, pinned and checksummed by
  `macos/build.sh`, no Xcode). Sparkle never checks on its own; the library's daily check stays as it is.
- On macOS the library's line reads "Version N is out — install"; clicking "install" makes Sparkle fetch the appcast,
  download the release's disk image, verify its EdDSA signature, replace the app and relaunch it. While that runs the
  line reads "Installing version N…". When the update cannot be installed (the app runs from the disk image or a
  folder the reader cannot write, the appcast is missing, the signature is wrong) the line falls back to "download",
  the site link, as before. No Sparkle window, no release notes, no setting.
- Linux keeps "download" and leaves updates to package managers; its look does not change.
- The release workflow signs the disk image with the EdDSA private key (a GitHub secret) and publishes `appcast.xml`
  as a release asset, so `releases/latest/download/appcast.xml` describes the latest release from the moment it is
  published. The private key lives in the maintainer's Keychain and in `~/keys/`, never in the repository; the public
  key is in `Info.plist`.
- The disk image's app carries the release's version in `CFBundleVersion` too (Sparkle compares that).

Decided with the maintainer (2026-10-20): the appcast is a release asset, CI signs with a secret, the whole install
lives in the library's line (no Sparkle windows, no bell icon, the same line on Linux and macOS).

## Non-Goals

Automatic installs without a click; checking from Sparkle's scheduler or a "Check for Updates…" menu item; release
notes in the app; delta updates; updates on Linux; a Developer ID or notarisation; updating Forzac 0.10.x itself (it has
no Sparkle: those readers download the version that brings it once more).

## Capabilities

### New Capabilities

### Modified Capabilities
- `update-check`: the line offers "install" on macOS and shows the install in progress; what the appcast and download
  requests carry; Forzac downloads the update itself on macOS.
- `macos-app`: the bundle embeds Sparkle, carries the EdDSA public key and the release's version in `CFBundleVersion`.
- `project-infrastructure`: a release also publishes the signed appcast for the macOS app.

## Impact

macOS shell: Sparkle linked and embedded (`macos/build.sh`, rpaths), a small user driver that drives the library's
line (`macos/`), `Info.plist` keys (`SUFeedURL`, `SUPublicEDKey`, `SUEnableAutomaticChecks`). Core: two strings in the
catalogue, nothing across the C ABI. Scripts: `scripts/package-macos.sh` (`CFBundleVersion`), a new
`scripts/appcast.sh`. CI: `.github/workflows/release.yml` (signing, `appcast.xml`), a secret `SPARKLE_ED_PRIVATE_KEY`.
Docs: `docs/publishing.md`. Licence: Sparkle is MIT; its licence goes into the bundle's `Resources/licenses`. Size: the
DMG grows by the framework (about 3 MB unpacked).

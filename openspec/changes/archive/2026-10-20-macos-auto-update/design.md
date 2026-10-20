## Context

`update-check` (0.10.0) asks GitHub's API once a day when the library opens and shows "Version N is out — download"
in the library's head; the link opens the site. The Mac app is built by `macos/build.sh` with `swiftc` (no Xcode
project), signed ad hoc, wrapped into a DMG by `scripts/package-macos.sh`, and the release workflow publishes the DMG
under the stable name `Forzac-macos-arm64.dmg`. There is no Developer ID and no notarisation
(`docs/macos-first-open.md`). The maintainer chose (2026-10-20): Sparkle 2, the appcast as a release asset, signing in
CI with a secret, and the whole install inside the existing line (no Sparkle windows, no bell icon).

## Goals / Non-Goals

**Goals:**
- One click on "install" in the library updates the Mac app in place and relaunches it.
- Updates are authenticated by an EdDSA key the maintainer owns; nothing depends on Apple certificates.
- The release stays one tag push; nothing new to deploy on the site.
- Linux, the core's check and the line's look stay as they are.

**Non-Goals:** Sparkle's scheduler, its windows, release notes, delta updates, signed appcasts, a menu item, Linux.

## Decisions

### Sparkle 2.10.0, pinned, unpacked by `macos/build.sh`
`build.sh` downloads `Sparkle-2.10.0.tar.xz` from Sparkle's GitHub release once into `macos/build/` (ignored by git),
checks its SHA-256 (`c2bf58aa…6da0c`, written in the script) and unpacks it to `macos/build/sparkle/`. That gives the
framework to link against and to embed, and `bin/sign_update` for the release. No Swift Package Manager or Xcode:
`swiftc` gets `-F macos/build/sparkle -framework Sparkle` and two rpaths, `@executable_path/../Frameworks` (the
bundle) and `@executable_path` (the bare `macos/build/Forzac` used for measurements, next to which a symlink
`Sparkle.framework` points into `sparkle/`). The framework is copied into `Forzac.app/Contents/Frameworks` with `ditto`
(keeps its symlinks), and the app is signed ad hoc as before.
*Alternatives:* vendoring the framework in git (3 MB binary in a source repository); SwiftPM (needs a package
manifest for an app that has none).

### The framework stays as Sparkle signed it
Sparkle's release is itself signed ad hoc (with the hardened-runtime flag), including `Autoupdate`, `Updater.app` and
the two XPC services. Forzac is not sandboxed and does not enable Sparkle's XPC services (`SUEnableInstallerLauncherService`
and `SUEnableDownloaderService` absent), so they are never launched; the framework is embedded untouched, so its
signatures stay valid and no re-signing order (innermost first) is needed. The outer `codesign --force --sign -` of the
app (not `--deep`) seals `Contents/Frameworks` as nested code; `codesign --verify --deep --strict` checks the whole tree.
Forzac has no hardened runtime, so library validation does not reject an ad-hoc framework.
*Alternative:* deleting the XPC services (Sparkle's documentation allows it for unsandboxed apps) — saves ~0.5 MB but
modifies the framework and forces re-signing it; not worth it.

### Why ad hoc signing is enough for Sparkle
From Sparkle 2.10's `SUUpdateValidator`: an app update is accepted when either the old and new EdDSA keys are the same
and the archive's EdDSA signature verifies (the code-signing identity may differ), or the Developer ID team matches.
With the first path the new app's code signature only has to be valid, not from the same identity; Sparkle's own error
text says "If no Apple Code Signing certificate is available, adhoc signing can be used at minimum". It also refuses an
update that drops the EdDSA key or the code signature, which ours never do. Downloads through Sparkle are not
quarantined and Sparkle clears quarantine from what it extracts, so the replaced app opens without the first-open step
again.

### Keys
`generate_keys --account forzac` makes the key pair: the private key goes into the maintainer's login Keychain and is
exported with `-x` to `~/keys/forzac-sparkle-ed25519` (mode 600, next to the Android upload key, outside every
repository); the public key goes into `Info.plist` as `SUPublicEDKey`. CI gets the exported private key as the
repository secret `SPARKLE_ED_PRIVATE_KEY`, set by the maintainer (or with their yes). Losing the private key means
the next release cannot update older apps; key rotation is Sparkle's (sign with the old key, ship the new public key).

### The appcast: a release asset made by `scripts/appcast.sh`
`scripts/appcast.sh X.Y.Z Forzac-X.Y.Z-macos-arm64.dmg` reads the private key from the environment
(`SPARKLE_ED_PRIVATE_KEY`, through `sign_update --ed-key-file -`) or else from the Keychain, and prints an RSS feed with
one item: `sparkle:version` and `sparkle:shortVersionString` X.Y.Z, `sparkle:minimumSystemVersion` 14.0, and an
enclosure `https://github.com/denisfl/forzac-desktop/releases/download/vX.Y.Z/Forzac-macos-arm64.dmg` with the length
and `sparkle:edSignature`. The enclosure names the versioned download URL, not `latest`, so a cached appcast never
points at a different file than the one it signed. `Info.plist`'s `SUFeedURL` is
`https://github.com/denisfl/forzac-desktop/releases/latest/download/appcast.xml`: a draft release changes nothing until
it is published, and then the appcast and the API's "latest" switch together. One item is enough: Sparkle only needs
the newest version, and older releases keep their own appcasts.
*Alternatives:* `generate_appcast` (wants a folder of every archive and writes its own URLs); the site (a deploy per
release).

### The update archive is the DMG
Sparkle installs from a DMG as well as from a zip; using the published `Forzac-macos-arm64.dmg` means one macOS asset,
and the signed bytes are the ones readers download by hand. The DMG has no licence agreement, so Sparkle's
`hdiutil attach` does not stop.

### Versions
Sparkle compares `CFBundleVersion`. `scripts/package-macos.sh` writes the release's version into
`CFBundleShortVersionString` and now into `CFBundleVersion` too; the committed `Info.plist` keeps its development values
(a development build has `CFBundleVersion` 1, which Sparkle's comparator ranks above any 0.x release, so a development
build never replaces itself with a release: it falls back to "download").

### Who checks: the core, as before; Sparkle only installs
The daily GitHub API check, its state keys and the dismissal stay in the core and are shared with Linux. Sparkle is
created lazily on the first click of "install" (`SPUUpdater` with our own user driver), never started at launch, with
`SUEnableAutomaticChecks` false in `Info.plist` (so it never asks the "check automatically?" question either) and
`userAgentString` "Forzac" (Sparkle's default would carry the app's and Sparkle's versions). The click calls
`checkForUpdates()`.

### A user driver that only talks to the line (shell, `macos/Update.swift`)
`SPUUserDriver` with every step answered at once: `showUpdateFound` → `.install`; `showReadyToInstallAndRelaunch` →
`.install`; download and extraction progress are ignored (the line says "Installing version N…" from the click until
the relaunch, nothing animates); `showUpdateNotFoundWithError`, `showUpdaterError` → acknowledge and tell the library to
fall back to "download". Sparkle then terminates the app through `NSApp.terminate` (`applicationWillTerminate` saves as
on Quit), installs with its `Autoupdate` tool and relaunches. If `SPUUpdater.start()` fails (the bare binary, a
misconfigured bundle) the line falls back at once. The library keeps one more state for the line, "installing" and
"failed" (shell-only, not remembered): failed shows "download" with the site link for the rest of the process.

What crosses the C ABI: nothing new. The two strings ("install", "Installing version {version}…") are in the core's
catalogue and reached through the existing `tr`.

## Risks / Trade-offs

- [macOS App Management (13+) asks before an app modifies another app's bundle] → Sparkle's installer is part of the
  app's own bundle and updates that same app. Checked on the M1 (macOS 14.5) with the app in `~/Applications` after the
  Terminal first-open step: no prompt. The Finder first-open step (quarantine kept, marked approved) in `/Applications`
  is in the release checklist (`docs/publishing.md`), since it needs the previous release installed as a reader would.
  If macOS asks, the reader allows once; if it refuses, the line falls back to "download".
- [The app is in `/Applications` but owned by another user / not writable] → Sparkle asks for an administrator's
  password through its own authorization prompt; refusing falls back to "download".
- [The private key leaks through CI] → the secret is only exposed to the tag-triggered release job's signing step;
  rotation is possible (see Keys).
- [Readers of 0.10.x] → they have no Sparkle and update by hand once more; the release notes say so.
- [GitHub's `latest/download` redirect] → Sparkle follows redirects (URLSession); the enclosure's host
  (`objects.githubusercontent.com`) is HTTPS.

## Migration Plan

The first release with Sparkle (0.11.0) is installed by hand; from then on "install" works. Before tagging it: the
secret exists. Rolling back a bad release: publish a newer fixed version (Sparkle never downgrades).

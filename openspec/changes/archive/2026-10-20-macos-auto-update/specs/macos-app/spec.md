## MODIFIED Requirements

### Requirement: Distribution
The Mac app SHALL be distributed as a disk image `Forzac-X.Y.Z-macos-arm64.dmg` downloaded from forzac.truenrth.app,
without a Developer ID, notarisation, the App Store or Homebrew. The image SHALL hold `Forzac.app` and a link to
`/Applications` for dragging it there, under the volume name "Forzac". The app SHALL be signed ad hoc as a whole
bundle (its resources sealed, so `codesign --verify --deep --strict` passes), with the bundle identifier
`app.truenrth.forzac`, the release's version in `CFBundleShortVersionString` and `CFBundleVersion`, and the app icon.
The bundle SHALL embed the update framework (`Contents/Frameworks/Sparkle.framework`, signed ad hoc as its release
comes) with its licence in `Contents/Resources/licenses`, and SHALL carry the EdDSA public key that update images are
verified with; the matching private key SHALL NOT be in the repository. A file `Forzac-X.Y.Z-macos-arm64.dmg.sha256` in
`shasum -a 256` format SHALL be published next to the image.

#### Scenario: Checking a download
- **WHEN** the reader runs `shasum -a 256 -c Forzac-0.2.0-macos-arm64.dmg.sha256` next to the downloaded image
- **THEN** it reports `OK`

#### Scenario: Installing
- **WHEN** the reader opens the image and drags Forzac onto the Applications link
- **THEN** Forzac is in `/Applications` with its icon, and after the first-open step it starts and opens books

#### Scenario: The bundle is sealed with the framework inside
- **WHEN** `codesign --verify --deep --strict Forzac.app` runs on the app from the image
- **THEN** it passes, and `Contents/Frameworks/Sparkle.framework` is there

## MODIFIED Requirements

### Requirement: What the request carries
The request SHALL be a plain HTTPS `GET` of the releases URL with no cookies, no query, nothing about the books, the
reading or the settings, and no Forzac version (macOS sends the user agent `Forzac`; Linux sends `curl`'s). Nothing
else SHALL be sent anywhere, except on macOS after the reader clicks "install": then the appcast and the disk image it
names are fetched over HTTPS from GitHub the same way — user agent `Forzac`, no cookies, no query, no system profile.

#### Scenario: The user agent on macOS
- **WHEN** the macOS app checks
- **THEN** the request's `User-Agent` is `Forzac`, without the app's or the system's version

#### Scenario: Installing sends nothing more
- **WHEN** the reader clicks "install"
- **THEN** the appcast and the disk image are requested with the user agent `Forzac` and no query, and nothing else is
  requested

### Requirement: The line in the library
While the remembered release is newer than Forzac and has not been dismissed, the library's head SHALL show, between
the brand and the buttons, one line in the muted interface colour at 13 px: "Version 0.9.1 is out — download"
(Russian "Вышла версия 0.9.1 — скачать") on Linux and "Version 0.9.1 is out — install" (Russian "Вышла версия 0.9.1 —
установить") on macOS, the last word in the link colour, followed by a small "×". "download" SHALL open
`https://forzac.truenrth.app/desktop/` in the browser; "install" SHALL install the update as "Installing on macOS"
says. The "×" SHALL hide the line and remember the dismissed version (`update-dismissed`); the line SHALL come back only
for a newer release. The line SHALL be hidden while searching and SHALL give way (be elided or hidden) when the head is
too narrow for it, never pushing the buttons. A check that finds a new release while the library is open SHALL show the
line at once. Forzac SHALL NOT download anything itself on Linux.

#### Scenario: Downloading
- **WHEN** the reader of the Linux archive clicks "download"
- **THEN** the browser opens forzac.truenrth.app/desktop/ and the line stays

#### Scenario: Dismissing
- **WHEN** the reader clicks the "×" of the 0.9.1 line
- **THEN** the line disappears, does not come back on later starts while 0.9.1 is the latest, and comes back when
  0.9.2 is released

## ADDED Requirements

### Requirement: Installing on macOS
Clicking "install" SHALL fetch the appcast of the latest release
(`https://github.com/denisfl/forzac-desktop/releases/latest/download/appcast.xml`), download the disk image it names,
verify the image's EdDSA signature against the public key in the running app, replace the app in place and relaunch it
in the new version, with no window or question of its own. While this runs, the line SHALL read "Installing version
0.9.1…" (Russian "Устанавливается версия 0.9.1…") in the muted colour, without a link or "×", and the rest of the
library SHALL keep working; the book windows SHALL be closed by the relaunch as by Quit, keeping every reading place.
When the update cannot be installed — the appcast or the image cannot be fetched, the signature does not verify, the
appcast offers nothing newer than the running app, the app runs from the disk image, a translocated copy or a folder the
reader cannot write to — the line SHALL turn back into "Version 0.9.1 is out — download" with the site link and "×",
for as long as the app runs, with no error dialog. Forzac SHALL NOT check, download or install anything until the
reader clicks "install".

#### Scenario: Installing an update
- **WHEN** Forzac 0.9.0 in `/Applications` shows "Version 0.9.1 is out — install" and the reader clicks "install"
- **THEN** the line reads "Installing version 0.9.1…", then Forzac quits and starts again as 0.9.1 without asking
  anything, and the next start shows no update line

#### Scenario: A forged update
- **WHEN** the disk image the appcast names is not signed with Forzac's key
- **THEN** nothing is installed and the line offers "download"

#### Scenario: Running from the disk image
- **WHEN** the reader opened Forzac straight from the mounted disk image and clicks "install"
- **THEN** nothing is installed and the line offers "download"

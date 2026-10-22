# update-check Specification

## Purpose
Telling the reader of the macOS app and the Linux release archives that a newer release is out: a daily request to
GitHub from the library and one quiet line there, from which the macOS app installs the update itself;
package-manager builds never ask.

## Requirements

### Requirement: When Forzac checks
When the library window opens and the last successful check is more than 24 hours old (or there has been none, or its
time is in the future), Forzac SHALL ask GitHub for the latest release, at most one request at a time per process. The
request SHALL run in the background: the library's first frame and its keys SHALL NOT wait for it, and it SHALL give
up after 10 seconds. A book window SHALL NOT start a check. Measurement runs (`--measure`, `--png` and the other
measurement flags) SHALL NOT check.

#### Scenario: First start
- **WHEN** Forzac starts for the first time and shows the library
- **THEN** it asks `https://api.github.com/repos/denisfl/forzac-desktop/releases/latest` once, in the background

#### Scenario: The same day
- **WHEN** the library opens again 3 hours after a successful check
- **THEN** no request is made and the line, if any, comes from what the last check found

#### Scenario: Offline
- **WHEN** the computer is offline or GitHub answers with an error or with something that has no release tag
- **THEN** nothing is shown or remembered, no error appears, and the next opening of the library tries again

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

### Requirement: Comparing versions
The answer's `tag_name` (`v0.9.1` or `0.9.1`) SHALL be read as dot-separated numbers and compared with the core's
version (the workspace version, 0.9.0 now) number by number, a missing number counting as 0. A tag that is not such
numbers (a pre-release such as `v1.0.0-rc1`) SHALL be ignored. A successful check SHALL remember its time and the
release's version in the state file (`update-checked`, `update-latest`).

#### Scenario: A newer release
- **WHEN** the latest release is `v0.9.1` and Forzac is 0.9.0
- **THEN** the line offers 0.9.1

#### Scenario: The same or an older release
- **WHEN** the latest release is `v0.9.0`, or Forzac is a newer development build than the release
- **THEN** no line is shown

### Requirement: The line in the library
While the remembered release is newer than Forzac and has not been dismissed, the library's head SHALL show, between
the brand and the buttons, one line in the muted interface colour at 13 px: "Version 0.9.1 is out — download"
(Russian "Вышла версия 0.9.1 — скачать") on Linux and "Version 0.9.1 is out — install" (Russian "Вышла версия 0.9.1 —
установить") on macOS, the last word in the link colour, followed by a small "×". "download" SHALL open
`https://forzac.app/desktop/` in the browser; "install" SHALL install the update as "Installing on macOS"
says. The "×" SHALL hide the line and remember the dismissed version (`update-dismissed`); the line SHALL come back only
for a newer release. The line SHALL be hidden while searching and SHALL give way (be elided or hidden) when the head is
too narrow for it, never pushing the buttons. A check that finds a new release while the library is open SHALL show the
line at once. Forzac SHALL NOT download anything itself on Linux.

#### Scenario: Downloading
- **WHEN** the reader of the Linux archive clicks "download"
- **THEN** the browser opens forzac.app/desktop/ and the line stays

#### Scenario: Dismissing
- **WHEN** the reader clicks the "×" of the 0.9.1 line
- **THEN** the line disappears, does not come back on later starts while 0.9.1 is the latest, and comes back when
  0.9.2 is released

### Requirement: Builds from a package manager
A Linux build whose data directory (`share/forzac`, next to the fonts) holds a file named `packaged` SHALL never check
and never show the line. The AUR packages `forzac` and `forzac-bin` and the `.deb` made by `scripts/package-deb.sh` SHALL
install that file; the release archives (`scripts/package-linux.sh`) and development builds SHALL NOT have it. The macOS
app SHALL always check.

#### Scenario: The .deb on a Raspberry Pi
- **WHEN** Forzac installed from `forzac-linux-arm64.deb` opens the library
- **THEN** no request is made

#### Scenario: The release archive
- **WHEN** Forzac runs from an unpacked `forzac-linux-x86_64.tar.gz`
- **THEN** it checks as on macOS

### Requirement: Speed and idle
The check SHALL NOT add to the library's first frame (measured with `--measure` as before), SHALL NOT draw anything
while it waits, and SHALL leave no process, timer or thread behind once it has answered or timed out.

#### Scenario: Idle after the check
- **WHEN** the check has finished and the library is left alone
- **THEN** Forzac uses no CPU, as before the change

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

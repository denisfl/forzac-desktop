## MODIFIED Requirements

### Requirement: The line in the library
While the remembered release is newer than Forzac and has not been dismissed, the library's "Books" SHALL show at the
top of its content, under the window's head (`library`) and above the views' tabs, one line in the muted interface
colour at 13 px: "Version 0.9.1 is out. Download" (Russian "Вышла версия 0.9.1. Скачать") on Linux and "Version 0.9.1
is out. Install" (Russian "Вышла версия 0.9.1. Установить") on macOS, the last word in the link colour, followed by
a small "×". It SHALL NOT be at the bottom of the window. "Download" SHALL open `https://forzac.app/desktop/` in the
browser; "Install" SHALL install the update as "Installing on macOS" says. The "×" SHALL hide the line and remember the
dismissed version (`update-dismissed`); the line SHALL come back only for a newer release. The line SHALL be hidden
while searching and SHALL be elided in a narrow window, never wrapping. A check that finds a new release while the
library is open SHALL show the line at once. Forzac SHALL NOT download anything itself on Linux.

#### Scenario: Downloading
- **WHEN** the reader of the Linux archive clicks "Download"
- **THEN** the browser opens forzac.app/desktop/ and the line stays

#### Scenario: Dismissing
- **WHEN** the reader clicks the "×" of the 0.9.1 line
- **THEN** the line disappears, does not come back on later starts while 0.9.1 is the latest, and comes back when
  0.9.2 is released

## ADDED Requirements

### Requirement: Checking by hand
The application menu SHALL have "Check for Updates…" ("Проверить обновления…"): on macOS in the Forzac menu under "About Forzac", on Linux in the library's main menu under "About Forzac", except in a build from a package manager, which never checks. It SHALL ask GitHub at once, whenever the last check was, as "What the request carries" says, and SHALL answer in a dialog:
- a newer release: "Version 0.9.1 is out." ("Вышла версия 0.9.1."), with "Install" ("Установить") on macOS, "Download" ("Скачать") on Linux, and "Later" ("Позже"); the library's line SHALL show again even if that version was dismissed;
- no newer release: "Forzac 0.9.0 is the latest version." ("Установлена последняя версия — 0.9.0.") with "OK";
- no answer in 10 seconds, or one that names no release: "Couldn't check for updates" ("Не удалось проверить обновления") with "GitHub didn't answer. Try again later." ("GitHub не ответил. Попробуйте позже.") and "OK".

A successful check SHALL be remembered as the automatic one is, so the next automatic check waits 24 hours from it. Measurement runs SHALL NOT show the item's dialog.

#### Scenario: A release out minutes after the last check
- **WHEN** the reader's Forzac 0.19.0 checked at 22:57, 0.20.0 was published at 23:05, and the reader chooses Forzac › Check for Updates… the next morning
- **THEN** a dialog says "Version 0.20.0 is out." with "Install" and "Later", and the library shows "Version 0.20.0 is out. Install"

#### Scenario: Up to date
- **WHEN** the reader of 0.20.0 chooses "Check for Updates…" and 0.20.0 is the latest release
- **THEN** a dialog says "Forzac 0.20.0 is the latest version."

#### Scenario: Offline
- **WHEN** the computer is offline and the reader chooses "Check for Updates…"
- **THEN** after at most 10 seconds a dialog says "Couldn't check for updates"

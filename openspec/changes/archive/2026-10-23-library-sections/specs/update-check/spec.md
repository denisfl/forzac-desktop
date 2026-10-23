## MODIFIED Requirements

### Requirement: The line in the library
While the remembered release is newer than Forzac and has not been dismissed, the library's "Books" SHALL show at the
top of its content, under the window's head (`library`) and above the views' tabs, one line in the muted interface
colour at 13 px: "Version 0.9.1 is out — download" (Russian "Вышла версия 0.9.1 — скачать") on Linux and "Version 0.9.1
is out — install" (Russian "Вышла версия 0.9.1 — установить") on macOS, the last word in the link colour, followed by
a small "×". It SHALL NOT be at the bottom of the window. "download" SHALL open `https://forzac.app/desktop/` in the
browser; "install" SHALL install the update as "Installing on macOS" says. The "×" SHALL hide the line and remember the
dismissed version (`update-dismissed`); the line SHALL come back only for a newer release. The line SHALL be hidden
while searching and SHALL be elided in a narrow window, never wrapping. A check that finds a new release while the
library is open SHALL show the line at once. Forzac SHALL NOT download anything itself on Linux.

#### Scenario: Downloading
- **WHEN** the reader of the Linux archive clicks "download"
- **THEN** the browser opens forzac.app/desktop/ and the line stays

#### Scenario: Dismissing
- **WHEN** the reader clicks the "×" of the 0.9.1 line
- **THEN** the line disappears, does not come back on later starts while 0.9.1 is the latest, and comes back when
  0.9.2 is released

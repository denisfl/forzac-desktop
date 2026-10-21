## MODIFIED Requirements

### Requirement: Desktop entry and MIME types
The packages SHALL install the desktop entry `app.truenrth.forzac.desktop`, which passes `desktop-file-validate`,
names Forzac in English and Russian, and declares `application/x-fictionbook+xml`,
`application/x-zip-compressed-fb2`, `application/x-fb3`, `application/epub+zip` and `text/markdown` — not `text/plain`,
which would offer a book reader for every log and configuration file. FB3, which shared-mime-info lacks,
SHALL be declared by `/usr/share/mime/packages/forzac.xml` (the `*.fb3` glob, a subclass of `application/zip`). The
Wayland app id of every Forzac window SHALL be `app.truenrth.forzac`, the desktop entry's name, so that taskbars and
docks tie the windows to the entry and its icon.

#### Scenario: A taskbar shows the icon
- **WHEN** a book is open in Forzac on the Pi 5's labwc desktop
- **THEN** the taskbar button of its window shows the Forzac icon, not a generic one

#### Scenario: An FB3 file is typed
- **WHEN** `xdg-mime query filetype book.fb3` is run after installing a package
- **THEN** it reports `application/x-fb3`

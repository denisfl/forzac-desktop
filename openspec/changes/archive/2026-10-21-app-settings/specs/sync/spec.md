## MODIFIED Requirements

### Requirement: The sync folder and files
Forzac SHALL keep its sync data in `.forzac/` inside the Books folder: one file per device, `<device>.sync`, where
`<device>` is 16 hex digits made once and remembered in the state, in the format of `docs/sync-format.md`. The file's
`device` line SHALL name the computer: on macOS the computer's name from Sharing settings, on Linux the host name. A
device SHALL write only its own file, replacing it atomically, and SHALL read every `.sync` file there; unreadable
lines and files SHALL be skipped.

#### Scenario: Two computers in one Dropbox
- **WHEN** the Mac and the Pi 5 use the same Books folder in Dropbox
- **THEN** `.forzac/` holds two `.sync` files, one written by each, and no conflicted copies appear

#### Scenario: Two Macs of one user
- **WHEN** the reader's two Macs, named "MacBook Air" and "Mac mini" in Sharing settings, sync one Books folder
- **THEN** the settings window on each lists the other by its name

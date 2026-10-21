## 1. Core

- [x] 1.1 Times `max(now, seen + 1)` for every local change; an applied place that differs is a local change
- [x] 1.2 The copied-device check (`sync.written`), a new id
- [x] 1.3 `sync` returns the books whose position it set; `forzac_synced` in the C ABI and `forzac.h`
- [x] 1.4 Tests: a clock ahead (position and removal), a copied state, moved books
- [x] 1.5 `docs/sync-format.md`: the time rule and the copied device

## 2. Shells

- [x] 2.1 macOS: resign active → save, sync; become active → sync, open books go to a moved place; quit → save, sync
- [x] 2.2 GTK: a book window's focus change → save, sync (and go on focus); shutdown → sync

## 3. Check

- [x] 3.1 `cargo test -p forzac-core`, `cargo clippy -p forzac-core`, `cargo fmt --check`, `macos/build.sh`; the GTK shell builds in `ci/fedora.Dockerfile` (no clippy in that image; the GTK shell was not run)
- [x] 3.2 With the phone (Nothing Phone (2), the folder moved by adb in place of Syncthing): read on the Mac, switch to another app, open on the phone at that place; read on the phone, come back to the Mac — the open book goes there

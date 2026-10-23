## 1. Core

- [x] 1.1 `core/src/log.rs`: the `reading-log` file (`read`, `finish`, `first` lines, escaping, lock, atomic replace), `Log::read`, `append` (this device's day, `time = max(now, time + 1)`), `finish` (30-day rule over every device), `first_open`, the first log with approximate finishes
- [x] 1.2 `log::Tally` (a port of the Android `ReadingTally`: 5-minute gap, midnight split by the offset, 3000 characters a minute, jumps, pause, drain) and `log::Reading` (a book's block starts in code points)
- [x] 1.3 Tests: a slow page, walked away, past midnight, turning forward, through the contents, flipping through, back, pause; the file survives a round trip and skips damaged lines; finishes (30 days, another device's finish, read again after a year); the first log
- [x] 1.4 `core/src/sync.rs`: `read` and `finish` records, keys, parsing and writing; the log into the own records, the others' newer days and missing finishes into the log in one write; tests on the `sync` scenarios (one day on two devices, read more the same day, a device gone, a finish from the phone, own id from another file not applied)
- [x] 1.5 The C ABI: `forzac_reading_event(doc, kind, at, offset)`, `forzac_state_log(state, doc, leaving, opened)` (append, the first opening); the finish with `forzac_state_save_position` when finished; `forzac.h`
- [x] 1.6 The settings' sync explanation in the catalogue; `docs/sync-format.md` (the two lines, their keys and merging)

## 2. Shells

- [x] 2.1 GTK: a `Reading` per book window; keys, clicks and scrolls → active; moves and jumps from `PageView`; focus loss, closing → paused; the log appended with the delayed save and on leaving; the first opening; the finish with the end page
- [x] 2.2 macOS: the same through the C ABI in `Reader` and `PageView` (jumps flagged), quitting appends

## 3. Check

- [x] 3.1 `cargo test -p forzac-core`, `cargo clippy -p forzac-core --all-targets -- -D warnings`, `cargo fmt --all --check`, `macos/build.sh`; the GTK shell builds and passes clippy in a container (`forzac-debian-clippy`; the log's GTK hooks were not run on Linux)
- [x] 3.2 On the Mac (dev build on a test state): read a few minutes, turn to the end page, switch apps; `reading-log` holds the day, the finish and the first opening; a sync folder with a phone-made file (`read`/`finish` lines) brings the phone's day and finish once — on the Mac (2026-10-22, test state): ten turns logged 3 s and 133 characters (the 3000-a-minute cap) under a new device id, the end page a finish with its first opening; the phone's lines by the core's tests (`the_phones_lines_arrive_once`), not through a real phone, which does not write them yet (Android `sync-reading-log` open)
- [x] 3.3 (not measured: archived without it at the maintainer's request, 2026-10-23) On the Pi 5 (with the maintainer's consent): 50 turns with `scripts/pi-headless.sh`, missed frames as before

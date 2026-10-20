## 1. Core: when a window shows a spread

- [x] 1.1 `Settings::spread(width, height)` and `MIN_SPREAD_COLUMN` (28 em); `page_style` gives the half-window page
      when it holds — unit tests in `core/src/settings/tests.rs`: 1440 × 900 at 20 px is a spread with 720 px pages,
      at 24 px not; 900 × 1440 and "Scroll" never; the note and table styles unchanged
- [x] 1.2 `forzac_settings_spread` in `core/src/ffi.rs` and `macos/forzac.h`; an FFI test that `forzac_paginate` at
      1440 × 900 gives the pages of a 720 px window and the count numbers the same pages
- [x] 1.3 `cargo fmt --all --check`, `cargo clippy -p forzac-core --all-targets -- -D warnings`, `cargo test -p
      forzac-core` green

## 2. GTK shell

- [x] 2.1 The style from the view's mode (`m` re-typesets in a spread window); in a spread the page kept is the left
      (even) one — relayout, turns by two, back into the previous chapter, chapter edges, `go_to_page`, jumps; the
      reading position stays where a resize found it
- [x] 2.2 Right pages cached with their offset (node and areas); the screen node of two pages for drawing, turns and
      `--png`; the end page centred; the empty half paper
- [x] 2.3 Both pages in `Tab` stops, clicks, the focus box, the search match bars, `shown_words`/`shown_rects` (selection
      across the gutter)
- [x] 2.4 Bookmarks: the shown range over both pages, the ribbon over the page holding the first bookmark (also while
      turning), the corner of the right page
- [x] 2.5 `cargo clippy --workspace --all-targets -- -D warnings` in the `forzac-debian-clippy` image green; under
      headless Weston in a container `--size 1920x1080 --png` shows two pages and `--turns 10` runs (`--size WxH`, a
      new measurement flag as the macOS shell has: measurement windows were always 760 × 980, never a spread)

## 3. macOS shell

- [x] 3.1 `PageView`: spread from `forzac_settings_spread`; the left page kept, turns by two, chapter edges, jumps, the
      position kept across resizes
- [x] 3.2 The spread bitmap (both pages, the right one at half the width) for `current`/`previous`, areas shifted, the
      end page centred
- [x] 3.3 `Tab`, clicks, the match bars, words and marks for the selection across both pages
- [x] 3.4 Bookmarks: the range, the ribbon's x on each layer, the right page's corner
- [x] 3.5 `macos/build.sh` (PATH with `~/.rustup/toolchains/stable-aarch64-apple-darwin/bin` first) builds without
      warnings

## 4. Documents

- [x] 4.1 `docs/android-parity.md` M6 and `docs/roadmap.md` 12a marked done; the help (`forzac_help` text) if it names
      the page layout; README if it lists the layouts

## 5. Verification on hardware

- [x] 5.1 macOS, M1 MacBook Pro full screen (1440 × 900): two pages with their own heads and feet; `l`/clicks turn
      spreads with the slide; a chapter with an odd page count ends with paper on the right; leaving and entering full
      screen keeps the line (screenshots); `Tab` on a spread, a selection across the gutter, `b` on a spread; 24 px
      shows one page; "Scroll" one column; idle 0 % CPU
- [x] 5.2 Raspberry Pi 5 at 120 Hz, 1920 × 1080: `scripts/pi-headless.sh` `--turns 20` across a chapter boundary
      without late frames, idle 0 CPU ticks — or recorded as not run if the maintainer releases without it

Checked 2026-10-20. 1: fmt, clippy (core on the Mac, the workspace with GTK in `forzac-debian-clippy`), the core tests
(164 unit, 6 integration). 2.5: headless Weston in `forzac-debian-clippy`, "Эти гениальные птицы" (FB2), `--size
1920x1080`: pages 3 and 4 of chapter 3 side by side, `--page 3` the same spread, `--turns 10` ran (software rendering,
31 Hz: 10 late frames of 67, not a measure of speed). 5.1 on the M1, the same book in a test home: in a 760 pt window
page 7 started "с приматами в интеллекте"; full screen showed pages 7–8 of 578 with that line on the left page; a click
turned to 9–10; leaving full screen showed page 9 holding the spread's first line, entering again 9–10 again; 0.01 s of
CPU in 10 s on a resting spread. With `--size 1440x900` and `--keys`/`--mouse`: `b` dropped the ribbon beside the left
page's column, a drag selected from the left page's last line to the right page's first with the bar, `G` showed the
chapter's page 29 alone with paper on the right and `l` the next chapter's 30–31, `h` from chapter 4 its predecessor's
last page alone, `Tab` outlined the left page's picture, `==` (24 px) one page, `m` one centred column. A note on the
right page was not tried (no notes in the book). Keys sent by the desktop-control tool did not reach the test window (a
second Forzac with the same bundle id was running); clicks did.

5.2: not run — the maintainer chose to archive without the Pi 5 measurement (2026-10-20). To measure later on the Pi:
`forzac book.fb2.zip --size 1920x1080 --chapter 5 --turns 20` in the headless labwc of `scripts/pi-headless.sh`.

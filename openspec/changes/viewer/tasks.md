## 1. Core

- [ ] 1.1 `Target::Image` and `Target::Table` with `is_viewable`; `Content::Row` keeps its row; areas for images at
      least a third of the column wide and one area per table per page or column stretch, inserted before its cells'
      areas; none in notes and previews; tests
- [ ] 1.2 `layout_table` and `Settings::table_style`: natural column geometry (longest word, at most 20 em), rows
      stacked, `Geometry::width` for the rules; tests: widths, wrapping, height, areas of markers in cells
- [ ] 1.3 C ABI: `ForzacArea::at`, kinds 2 and 3, `forzac_image`, `forzac_table` / `forzac_table_page` /
      `forzac_table_free` over a shared `Published`; `macos/forzac.h`; ABI tests
- [ ] 1.4 Corpus test: every table typeset for the view, the slowest printed

## 2. GTK shell

- [ ] 2.1 Page view: viewable areas in `Tab` stops (skip repeats), the outline for a focused image or table, clicks
      prefer markers and links, `Open::Image` / `Open::Table`; `open`, `jumps`, `texture` and `table_view` for the
      view
- [ ] 2.2 `viewer.rs`: the image view (fit, zoom ×1.25 to ×5, pan, clamp, 150 ms movements, wheel, touchpad, drag,
      pinch, `Ctrl`+wheel, click closes, close button, fades)
- [ ] 2.3 The table view: typeset per 5 % step and theme, padding, centring, markers and links (click, `Tab`, `Enter`),
      a jump closes the view; the note sheet returns the focus to the view
- [ ] 2.4 `main.rs`: open the view from the page, redraw it on a theme change; help and README

## 3. macOS shell

- [ ] 3.1 Page view: kinds 2 and 3, `Tab` stops, the outline, clicks prefer markers and links, `Command.image` /
      `.table`
- [ ] 3.2 `Viewer.swift`: the image view (a layer, Core Animation movements, wheel, trackpad, drag, pinch, `⌘`+wheel,
      click closes, close button, fades)
- [ ] 3.3 The table view: tiles, re-typesetting per step, markers and links, a jump closes the view; `NoteSheet`
      returns the focus to the view; `build.sh`
- [ ] 3.4 Help and README

## 4. Verification

- [ ] 4.1 `cargo fmt --all --check`, clippy, `cargo test -p forzac-core`, the corpus test, both shells build
- [ ] 4.2 By hand on the Pi 5 (GTK): a schematic and a wide table, keys, mouse, both page modes, a note in a cell,
      idle CPU with the view open; `scripts/pi-headless.sh` still 0 missed frames and 0 idle CPU; the table timing
- [ ] 4.3 By hand on the M1 Mac: the same with the trackpad
- [ ] 4.4 Results in `docs/android-parity.md` (V1, V2) and the roadmap; archive

## 1. Core

- [x] 1.1 (2026-10-24) `Margins::column`: the measure in em and the share of the page's width (0.04 / 0.07 / 0.13); `PageStyle::column` takes the share; `spread` and `spread_inset` use the share of the half; `for_window` the medium 7 % — check: `settings` tests for 500 px (20 / 35 / 65 px), 1000 px (the measure, 160 px) and the 1440 px spread (619.2 px column)
- [x] 1.2 (2026-10-24) Strings "Size", "Alignment", "Font" with their Russian — check: the catalogue test passes once the shells ask for them
- [x] 1.3 (2026-10-24) `cargo test -p forzac-core`, `cargo clippy` (the workspace in the Debian container), `cargo fmt --all --check` pass

## 2. macOS

- [x] 2.1 (2026-10-24: `--panel settings --png` at 760 × 980 in Light (ru), Wave (en), Dark (ru), at 440 × 800 in Sepia, and in a 1440 × 875 spread from a copied state) `SettingsSheet`: the grid, captions, theme cards with the name in the caption, the size buttons without a number, equal segments with dividers, the soft selection, `ChipsRow` (stretch when they fit, scroll when not) — check: screenshots, both edges of every row line up
- [x] 2.2 (2026-10-24: `--keys jjjll` outlines the font row and picks Source Serif; `--keys jhhh` dims the smaller "A"; at 440 px `--keys jjjlll` scrolls PT Sans into view) Keys: the same seven controls in the same order — check: `--keys` screenshots

## 3. GTK

- [x] 3.1 (2026-10-24: the `forzac-debian-dev` container under a headless Weston (pixman), 760 × 980 in Light (ru), Wave (en), Dark (ru), 440 × 800 in Sepia; the panel opened by a temporary, uncommitted hook in `main.rs`) `settings_sheet.rs` and the panels' CSS: the same layout and colours as the Mac's — check: screenshots, both edges of every row line up
- [ ] 3.2 Keys and clicks in the GTK sheet on a real Linux desktop (no input injection under Weston; the shortcuts' code did not change) — check: by hand on the Pi 5 or Omarchy

## 4. Check

- [ ] 4.1 The maintainer looks at both shells and decides on the release

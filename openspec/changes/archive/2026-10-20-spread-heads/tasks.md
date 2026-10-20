## 1. Core

- [x] 1.1 `PageStyle::spread`, set by `Settings::page_style` on a spread; `ChapterLayout` keeps the book's title;
      `render_numbered` puts it over even pages above 0 of a spread — a unit test: a spread's page 2 has the book's
      title in its head, page 3 the chapter's, a single page 2 the chapter's
- [x] 1.2 `cargo fmt --all --check`, `cargo clippy -p forzac-core --all-targets -- -D warnings`, `cargo test -p
      forzac-core` green

## 2. Shells

- [x] 2.1 macOS `PageView.updateRibbon`: on a spread the right page's offset
- [x] 2.2 GTK `PageView::ribbon_dx`: the same; `cargo clippy --workspace --all-targets -- -D warnings` in the
      `forzac-debian-clippy` image green

## 3. Verification on hardware

- [x] 3.1 M1, a 1440 × 900 window: a spread's heads (book left, chapter right), `b` on the spread drops the ribbon
      beside the right page's column (screenshots); a single page keeps the chapter's head

Checked 2026-10-20 on the M1. 1: `cargo fmt --all --check`, core clippy, the core tests (174 unit with
`heads_of_a_spread`, 5 + 1 integration). 2.2: workspace clippy in `forzac-debian-clippy` green; the GTK shell was not
looked at on the Pi (same core rendering; the ribbon's offset is two lines). 3.1: "Эти гениальные птицы" at
`--size 1440x900 --keys b`: the left head "Эти гениальные птицы", the right head the chapter's title, the ribbon beside
the right page's column; at 760 × 980 a single page keeps the chapter's head.

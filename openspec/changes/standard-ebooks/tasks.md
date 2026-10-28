## 1. Spike

- [x] 1.1 resvg 0.49 without text and system fonts on the M1 (release): "Alice"'s `cover.svg` (a JPEG in `data:`, a
  `<style>`, a translucent box) and the heaviest Tenniel illustration at 1336 px — check: both drawn right, times and
  peak memory in design.md — ✓ 2026-10-28: cover 169 ms, illustration 115–124 ms, a "Flatland" diagram under 1 ms,
  peak 68 MB; over Android's 100 ms threshold, so rasters are cached (decision 4)

## 2. Parsing (core)

- [x] 2.1 SVG images outside the spine and SVG covers; SVG recognised by content when the manifest says otherwise —
  check: `svg_images_and_cover`, `svg_document_in_the_spine_is_read_as_a_document`,
  `mislabelled_images_are_recognised_by_content`
- [x] 2.2 `⸺ ⸻ ℅` replaced in text (styles kept) and in the contents — check: `rare_dashes_replaced`
- [x] 2.3 Verse and diaries by `epub:type` — check: `verse_by_epub_type`, `diary_is_text_letter_inside_is_quote`
- [x] 2.4 The title page with an image keeps only its images — check: `titlepage_with_image_is_a_picture`,
  `titlepage_without_image_keeps_its_text`
- [x] 2.5 `p` in `hgroup` a heading one level lower (else a subtitle); `hidden` skipped — check:
  `hgroup_subtitle_is_a_lower_heading`, `hidden_is_not_shown`
- [x] 2.6 A list item's marker with its nested paragraph — check: `list_marker_joins_the_item_paragraph` (already so)
- [x] 2.7 Verse indents `i1`…`i9` as U+2003 — check: `verse_line_indent` (prose is not indented)
- [x] 2.8 `Style::SmallCaps`; CSS small-capitals classes and the `b`/`strong` rules — check:
  `small_caps_from_standard_ebooks_css`, `small_caps_class_and_bold_small_caps`, `plain_bold_stays_bold`
- [x] 2.9 Corpus: `standard_ebooks` in `core/tests/corpus.rs` over `se-*` pairs under `FORZAC_CORPUS` — check:
  `FORZAC_CORPUS=../forzac/samples/epub3 cargo test --release -p forzac-core --test corpus` — ✓ 2026-10-28: 9 pairs,
  advanced = compatible in chapters, contents, images and text (invisible characters aside); Keats 133 chapters and
  786 verse paragraphs (812 `p` in verse less 25 `hgroup` subtitles and 4 epigraphs), "Alice" 44 images and an SVG
  cover, "Dracula" 268 citations, "Flatland" "I. Of the Nature of Flatland" and small capitals; no lone "•"; all 68
  EPUBs of the corpus open

## 3. Images and typesetting (core)

- [x] 3.1 `images`: `is_svg`, `size`, `raster` (fit the box, at most 4× and 4096 px, cache 128 MB, one drawing at a
  time, nothing outside the book), `png`, `is_ink` — check: `core/src/images/tests.rs`
- [x] 3.2 The typesetter sizes SVG by `images::size` — check: existing image tests, SVG pages on the Mac
- [x] 3.3 Small capitals: `Shaper::has_small_caps`, `smcp` shaping, reduced capitals in a copy of the text — check:
  `small_capitals_of_the_font_or_reduced_capitals` (PT Serif reduced at the same offsets, Literata its glyphs)
- [x] 3.4 Verse breaks per line, wrapped parts `(n + 1)` em in; U+2003 as a 1 em box — check:
  `a_long_verse_line_goes_on_one_em_right_of_its_start`; the first-line indent only on the first line —
  `the_first_line_indent_is_the_first_lines_only`
- [x] 3.5 The C ABI: `forzac_svg_raster`, `forzac_raster_free`, `forzac_is_ink`, `forzac_warm_images`; `forzac_cover`
  an SVG cover as PNG; `forzac.h` — check: `svg_images_and_an_svg_cover`

## 4. Shells

- [x] 4.1 macOS: `Picture` (image and ink flag), SVG through the core, line art drawn in the ink (pages, note panel,
  table tiles, a tinted image in the view, again on a theme change), SVG of nearby chapters drawn ahead — check:
  `macos/build.sh`; screenshots below
- [x] 4.2 GTK: `Picture`, SVG through the core as a texture over the raster's bytes, line art by a mask node (pages and
  the view), SVG covers as PNG for thumbnails and the note panel — check: `cargo clippy --workspace --all-targets -- -D
  warnings` in `forzac-debian` (arm64) — ✓ clean; not run on Linux (see 5.3)

## 5. Check

- [x] 5.1 `cargo test -p forzac-core`, `cargo clippy -p forzac-core --all-targets -- -D warnings`, `cargo fmt --all
  --check` — ✓ 2026-10-28: 320 tests
- [x] 5.2 On the Mac (`--png`, no reading state): "Flatland" advanced — the diagram light in the Dark theme on the page
  and in the view, the dedication's small capitals (reduced capitals in PT Serif); "Alice" advanced — Tenniel's
  frontispiece; Keats — verse indents; the library (a test state) — SVG covers of "Alice", "Flatland", Keats and "Pride
  and Prejudice" with their titles — ✓ 2026-10-28. Turns not measured: `--turns` recorded no animation frames
- [ ] 5.3 On the Raspberry Pi 5 (GTK): the same books, turning through "Alice"'s illustrations without missed frames —
  not done: the Pi did not answer on the network on 2026-10-28, and a release build in the Linux container stopped
  OrbStack (the Mac's disk was full)
- [x] 5.4 `openspec validate standard-ebooks --strict` — ✓ valid
- [ ] 5.5 Archive after 5.3 or the maintainer's waiver

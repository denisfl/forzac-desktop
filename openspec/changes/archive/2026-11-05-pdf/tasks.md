## 1. Decision and spike

- [x] 1.1 The maintainer's word on PDFium for Linux (design, decision 3): taken on 2026-10-30
- [x] 1.2 Spike: a 640-page PDF and a scan rendered on the M1 (PDFKit) and in the Linux container (`pdfium-render`) — check: first page and per-page times in design.md

## 2. Core

- [x] 2.1 `Format::Pdf` by signature, in `forzac_open` and the C ABI — check: a test with a `%PDF-` file, a test with a text file naming PDF
- [x] 2.2 Book id, state, books folder and library entries for a PDF, with cover and title given by the shell — check: core tests
- [x] 2.3 Reading log lines from a PDF; the sync file without its place — check: core tests

## 3. macOS

- [x] 3.1 The PDF window on PDFKit: pages on white, fit, spread, "Slide", keys, clicks, wheel, foot — check: `--png`, screenshots
- [x] 3.2 (2026-11-05: the maintainer checked the build on the Mac before 0.20.0) Zoom and pan, sharp after the gesture — check: `--keys`, by hand for the pinch. Keys checked; the pinch and the touchpad by hand remain
- [x] 3.3 Menu, contents from the outline, search, links, the Back pill, in the Light chrome — check: screenshots in Dark and Light

## 4. Linux

- [x] 4.1 The same on PDFium in the GTK shell — check: `cargo clippy`, under headless Weston
- [x] 4.2 `libpdfium` in the `.deb`, tarballs and PKGBUILD — check: installs and opens a PDF in `ci/` containers. Debian arm64: the `.deb` and the tarball; Arch x86_64: both AUR packages build and install, `forzac-bin` opens one under headless sway at four scales (`ci/arch-check.sh book file.pdf`)

- [x] 4.3 A found word marked on its page, a line under links, the author from the metadata, a double click zooms, in both shells (`docs/android-parity.md` A7–A10) — check: core tests; `--query`, `--pick`, `--click`, screenshots, on a rotated and a cropped page too

## 5. Check

- [x] 5.1 `cargo test -p forzac-core`, `clippy`, `fmt`; `openspec validate pdf --strict`
- [x] 5.2 (2026-11-05: released in 0.20.0 without it, by the maintainer's word; the Pi 5 waived) Measure the Speed requirement on the Mac; the Pi 5 when the maintainer lifts its waiver. Mac: 20 turns of a text PDF at 120 Hz, 475 frames, 4 late (max 13.4 ms), against 3 of 463 (max 14.3 ms) for a book; idle 0.04 s of CPU in 10 s. The Pi remains
- [x] 5.3 (2026-11-05: docs/releases/0.20.0.md) `docs/android-parity.md` F5 and NP1, README, release notes; archive. Parity and README done; the release notes and the archive wait for the release

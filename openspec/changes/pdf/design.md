## Context

Android renders with the system's `PdfRenderer` (PDFium) and PDFKit is on both Apple platforms, so no platform needs a PDF engine written for it. The Rust core emits positioned glyph runs and has no renderer of pages; a PDF does not fit its typesetting and needs no typesetting. Android's rules: `forzac/openspec/specs/pdf-reading/spec.md`.

## Goals / Non-Goals

**Goals:** a PDF opens and reads comfortably on the Mac and Linux, as on paper, with the book's keys.

**Non-Goals:** themes on PDF pages; margin cropping, spread splitting and screens by lines; selection and highlights in a PDF; syncing the place; PDF in the core.

## Decisions

1. **Each shell renders with its platform's engine; the core only recognises the type.** `%PDF-` at the start of the file (within the first KiB, as readers accept) gives a distinct `Format::Pdf`, and the shell opens its PDF window. Rejected: PDF in the core through PDFium on all platforms, which would ship a second engine on Apple platforms and give up PDFKit's search and links.
2. **macOS: PDFKit** (`PDFDocument`, `PDFPage.draw`, `findString`, `PDFOutline`, annotations for links). Pages are drawn by our view, not `PDFView`, so turning is the same Core Animation slide as a book's.
3. **Linux: PDFium** (the maintainer's decision, 2026-10-30). Through `pdfium-render`, with `libpdfium.so` from `bblanchon/pdfium-binaries` shipped in our packages, the crate's feature and the library's build pinned together per release. Its permissive licence keeps the Linux binary under the MPL; and it is the only candidate that is both permissive and ready for everything the spec asks. The price is a third-party C++ binary that adds about two thirds to the Linux download and whose security updates become our job. Rejected: Poppler and MuPDF, which would put the whole Linux binary under the GPL or AGPL and collide with the LPPL patterns built into the core; hayro for now, which renders well and fast but has no text, link or outline API, and says itself that robustness is unfinished. The evidence is in "PDF engine for Linux" below.
4. **The place is `Locator(0, page, 0)`**, as on Android, in the same state file. The sync file does not carry it: another device's format and layout are irrelevant here, and Android's rule is the same.
5. **Rendering off the main thread, one page at a time, neighbours ahead**; a rendered page is cached at its window size. Zooming draws the cached page scaled at once and re-renders the part in view sharply when the gesture ends: a whole page at 4× would take hundreds of megabytes.
6. **Light chrome over a PDF.** The PDF window passes the Light palette to the shared menu and panels without touching the saved theme.

## PDF engine for Linux

Checked on 2026-10-30. "Measured" is the spike below, "source" is a project's own file or a package index, linked at the end; anything else is marked as an assumption.

| | PDFium + `pdfium-render` | Poppler + `poppler-rs` | MuPDF + `mupdf` | hayro |
|---|---|---|---|---|
| **1. Licence** (source) | BSD-3-Clause and Apache-2.0 in one `LICENSE`; crate MIT or Apache-2.0; 18 bundled licences to ship (168 KB) | GPL: "any application using poppler must also be licensed under the GPL"; the MIT binding does not change that | AGPL-3.0-or-later or a paid licence from Artifex; the crate is AGPL too | Apache-2.0 or MIT |
| **Our Linux binary would be** | MPL-2.0 | GPL-3.0 | AGPL-3.0 | MPL-2.0 |
| **2. 1,209 pages in 18 files, every page** (measured) | All render, none blank, 19.3 s | All render, none blank, 30.9 s | All render, none blank, 36.7 s | All render, none blank, 17.4 s |
| **Look against PDFium** (measured, pixels off by more than 64/255) | Reference | 0.3–2.8%: other anti-aliasing, no faults seen | 0–0.8%: nearly identical | 1–3%: thin curves visibly kinked and uneven small-text spacing on 2 of 4 pages viewed |
| **File missing its last 3 KB** (measured) | Opens, 640 pages | Refused, "damaged" | Opens, 640 pages | Opens, 640 pages |
| **File cut in half** (measured) | Refused, clean error | Refused, clean error | Opens with no pages, no error | Refused, clean error |
| **3. Search text** | Ready; matches Poppler and MuPDF on a Cyrillic count (measured) | Ready (measured) | Ready (measured) | No API: to be built on glyph runs and their Unicode (source) |
| **Links, outline** | Ready: 85 links, 41 entries (measured) | Ready: 85, 41 (measured) | Ready: 85, 41 (measured) | No API (source: nothing in 0.8.0) |
| **Title, password** | Ready; password reported as such (measured) | Ready; reported (measured) | Ready; reported (measured) | Ready; reported (measured) |
| **4. Speed** | Table below | Three times PDFium on the heavy scan, faster on a light one | Slower on scans, most memory | Fastest on the heavy scan and on the whole set; slowest open |
| **Threads** | "Assumed not to be thread safe"; the crate serialises every call behind a lock (source) | Not checked | Not checked | Rust, no global lock in the API; not tried |
| **5. What we ship** | `libpdfium.so`: 8.1 MB, 6.6 stripped, 3.3 in gzip, 2.4 in xz (measured, arm64); needs only libc, libm, libgcc, GLIBC 2.17 on arm64 | Nothing, a dependency; 86 linked libraries in the spike against 5 (measured) | Nothing separate; spike binary 8.5 MB stripped against PDFium's 3.1 (measured) | Nothing separate; spike binary 5.9 MB stripped (measured) |
| **Where it comes from** | Unofficial weekly CI builds, x64 and arm64 glibc; no package in Debian 13 or Fedora or official Arch (source); the AUR was not reachable | Debian 13: 25.03; Fedora: 26.01; Arch: 26.10 (source) | Vendored C, compiled by `cargo` (50 s on 12 cores); Debian 13: 1.25; upstream 1.28 | crates.io |
| **Security updates** | Ours, with a release | The distribution's; the strongest point for Poppler | Ours, with a release | Ours, with a release; `forbid(unsafe_code)` in `hayro` itself lowers the stakes |
| **6. Project** | Chromium's engine; last build 2026-10-26; crate last pushed 2026-10-08, 1.76 M recent downloads | 26.10.0 of 2026-10-04; binding 0.26 of 2026-02-21, 46 K downloads, its README says "work-in-progress" | Commits on 2026-10-29; Artifex; crate 0.8.0, June 2026 | Daily commits, but pre-1.0 and 1897 commits by one person; API "still" being worked out, by its README |

Also dropped: `pdf.js`, it needs a JavaScript engine; Ghostscript, same AGPL as MuPDF and no page-viewer API; Evince's library and `xpdf`, GPL as Poppler; Qt's `QtPdf`, PDFium with Qt on top; `pdf-render`, 2.6 K recent downloads, not tried.

**Render times, ms**, median of three runs of 30 sampled pages; a page 1440 px high unless noted; includes a copy to RGB. Debian 13 arm64 in Docker on the M1 (12 CPUs given), one thread, no cache of ours. PDFKit runs natively on the same Mac, for reference.

| | Text, 640 pages: first / median / p95 / at 4320 px | Scan, 38 pages of JPEG: first / median / p95 / at 4320 px | Mixed, 225 pages: median / p95 | Text extraction, per page | Peak memory, text / scan |
|---|---|---|---|---|---|
| PDFium | 49 / 6.5 / 11 / 41 | 65 / 60 / 72 / 361 | 5.0 / 10 | 1.6 | 258 / 346 MB |
| Poppler | 304 / 5.0 / 12 / 31 | 196 / 196 / 231 / 395 | 7.0 / 21 | 1.7 | 145 / 206 MB |
| MuPDF | 146 / 6.4 / 81 / 30 | 165 / 162 / 173 / 489 | 5.1 / 17 | 3.1 | 229 / 450 MB |
| hayro | 49 / 8.3 / 9.1 / 42 | 47 / 46 / 52 / 179 | 7.2 / 9.8 | none | 131 / 245 MB |
| PDFKit (M1, native) | 59 / 1.7 / 6.1 / 28 | 62 / 46 / 52 / 148 | 17 / 68 | not measured | not measured |

A lighter scan (39 pages, 20 MB, one run) orders them differently: Poppler 17, PDFium 22, hayro 28, MuPDF 74 ms, so a scan's encoding matters as much as the engine. MuPDF's p95 on text moved between 19 and 105 ms across runs; the other numbers held within about 20%, not a rigorous bound.

**Not measured:** nothing on the Pi 5, which was unreachable, and no frames: these are page times in a VM, not turns. No scan in JBIG2, JPEG 2000 or CCITT was among the test files, and no CMYK or transparency-heavy file was picked on purpose, so those remain open for every engine.

**Conclusion.** No engine fails on ordinary books, and every one takes far longer than a 120 Hz frame on a scan, so "Speed" rests on rendering ahead in any case (decision 5); speed does not decide this. The licence does, then the spec:

- The MPL does not forbid Poppler or MuPDF. Its section 3.3 allows a larger work under GPL or AGPL, our source files staying MPL as well, and the repository has no "Incompatible With Secondary Licenses" notice. So it is a price and not a ban: the Linux binary would be GPL or AGPL while the macOS one stays MPL, and GPL-3.0 rather than 2.0, because `zopfli` is Apache-2.0 only.
- There is a harder problem. The Russian hyphenation patterns are LPPL 1.2 or later and are built into the binary (`core/src/hyphen.rs`). The FSF lists the LPPL as incompatible with the GPL. Poppler or MuPDF would first need those patterns out of the binary or replaced. *Assumption, not legal advice:* a data file loaded at run time would count as aggregation.
- hayro is the one real alternative, and worth another look in a year: no library to ship, memory-safe in its own crate, fastest on the heavy scan. Today we would write search, links and the outline ourselves over an unsettled API, accept its own warning of "timeouts and OOM errors" on untrusted input, and see thin curves worse than the others'.
- PDFium's cost is real: the weight, an unofficial builder, and security updates that come only when we release, since no distribution packages it for us.

Corrections to what this document assumed earlier: PDFium's licence is BSD and Apache, not just BSD; "pages match Android" holds only roughly, since AOSP's copy of PDFium was last merged in early 2025 and the Mac and iOS use PDFKit anyway.

**Checked in implementation:**
- One thread owns PDFium and everything else asks it (`gtk/src/pdf.rs`). Search reads a page at a time, below the frame clock's priority: a turn waits for at most one page's text.
- The crate's `pdfium_latest` (build 7881) with library build 8125 passes every call the shell makes, on the files above. The pair is pinned in `scripts/pdfium.sh` and the source `PKGBUILD`; a new build is to be tried against them.
- The x86_64 library needs the same libc, libm, libpthread and libgcc as the arm64 one.
- PDFium marks a hyphen it took for a line's end with U+FFFE or U+0002; the page search removes them.

**Still an assumption:** a Pi 5 runs these pages about three to four times slower than the M1, so about 200–250 ms for PDFium on this scan. To be measured when the waiver lifts.

**Packaging, as built:** `lib/forzac/libpdfium.so`, stripped, beside `bin/` in the tarballs: arm64 11.4 MB against 7.0 before (+63%). `/usr/lib/forzac/` in the `.deb`: 7.7 MB against 4.3 (+79%). The library accounts for 3.3 MB in gzip and 2.4 MB in xz; the rest is `pdfium-render` and its dependencies in the binary, and the licences. `forzac-bin` copies `lib` too; `forzac` adds per-architecture `source_*` with checksums. The licences go to `share/licenses/forzac/pdfium` and Debian's `copyright`. Bound at run time by path from the executable, so a missing library breaks PDFs and not books. The macOS build adds only the system's PDFKit.

**Sources:**
[PDFium LICENSE](https://pdfium.googlesource.com/pdfium/+/refs/heads/main/LICENSE),
[pdfium-render](https://github.com/ajrcarey/pdfium-render) (`README.md`, `LICENSE.md`, `Cargo.toml`),
[pdfium-binaries](https://github.com/bblanchon/pdfium-binaries) (README, [releases](https://github.com/bblanchon/pdfium-binaries/releases)),
[AOSP PDFium](https://android.googlesource.com/platform/external/pdfium/),
[Poppler README](https://gitlab.freedesktop.org/poppler/poppler/-/blob/master/README.md), [COPYING](https://gitlab.freedesktop.org/poppler/poppler/-/blob/master/COPYING), [releases](https://poppler.freedesktop.org/),
[poppler-rs](https://gitlab.gnome.org/World/Rust/poppler-rs),
[Arch poppler-glib](https://archlinux.org/packages/extra/x86_64/poppler-glib/),
[MuPDF README](https://github.com/ArtifexSoftware/mupdf/blob/master/README), [COPYING](https://github.com/ArtifexSoftware/mupdf/blob/master/COPYING), [Artifex licensing](https://artifex.com/licensing),
[mupdf-rs](https://github.com/messense/mupdf-rs),
[hayro README](https://github.com/LaurenzV/hayro/blob/main/README.md),
[MPL 2.0](https://www.mozilla.org/en-US/MPL/2.0/) (sections 1.12, 3.3),
[FSF licence list](https://www.gnu.org/licenses/license-list.html) (LPPL, MPL, Apache).

## Risks / Trade-offs

- [PDFium is a third-party binary we ship and must update; no distribution does it for us] → Pin a build per release, check its checksum, and release out of turn for a PDFium security fix; the update check and packages already carry a version per architecture.
- [A prebuilt library from one unofficial builder] → If it stops, the same `build.sh` steps run in our CI; the crate binds any compatible build.
- [Heavy scans render slowly on the Pi 5] → Measure on the Pi; a page late to the end of a turn shows the cached neighbour's size in white first rather than holding the turn.
- [Search on a large PDF] → Text is extracted in the background on the first search, results arrive as they are found, as on Android.

## Migration Plan

New feature; no data changes. Rolling back leaves PDFs on the shelf that the older version refuses with its usual message.

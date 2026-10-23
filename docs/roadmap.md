# Roadmap

From the spike ([docs/spike.md](spike.md)) to 1.0. Each row is an OpenSpec change in `openspec/changes/<name>`
(`proposal → specs → design → tasks`, then implementation and `openspec archive`). Sizes are rough, for one developer,
and get refined in each change's design.

Linux comes first: Omarchy and the Raspberry Pi 5 are the primary audience. macOS follows once the reader is complete
on Linux, so the shared core API is settled before the second shell catches up.

**Status (2026-10-19).** Milestones 0.2–0.4 are done and archived (2026-10-16–17), and so are `localization`,
`bookmarks`, `selection-quotes`, `library`, `reader-menu`, `help-card` and (2026-10-20) `two-page-spread` from 1.0.
Left for 1.0: `release-1-0` (proposal only). Nothing is published yet; the 0.1.0 release was dry-run on 2026-10-17
([publishing.md](publishing.md)).

**After 0.9.0 (released 2026-10-19).** The maintainer's direction is a Calibre-like library: first the Books folder
(`books-folder`, done), then the library over it, sending books to a Kindle (FB2 → EPUB and Send to Kindle) and,
later, syncing with the Android app through Dropbox-like services. `update-check` is done too, and on macOS
`macos-auto-update` installs the update from the library's line (Sparkle; from 0.11.0).

## After 0.11.0 — a library and syncing, after Calibre

Agreed with the maintainer on 2026-10-20: "follow calibre-ebook.com — a library and syncing data with readers". In this
order; the library's screens (17–19) are designed after the UX review (`docs/ux-review.md`). Done: `book-identity`,
`folder-import`, `library-metadata`, `library-shelves`, `metadata-editing`,
`books-folder-setting` and `send-to-kindle` (2026-10-20), released in 0.12.0 the same day; `sync-format` and
`sync-handover` (format: [sync-format.md](sync-format.md); the Android side: `sync-folder`, `sync-clock`), released in
0.13.0 on 2026-10-21. Then `app-settings` (2026-10-21, for 0.14.0): the settings window with the language and syncing,
as the Android app's. Then `reading-log` and `reading-stats` (2026-10-22): the reading log, synced with the phone, and
the stats window with the end page's totals, as the Android app's 0.9.10. Then `library-sections` (2026-10-23): "Books"
and "Reading stats" as sections of the library window, as the Android app's 0.9.11, placed by each platform's guidelines
(a toolbar segmented control on macOS; the header on Linux, a bottom pill under 550 px), the theme in the settings.

| # | Change | What | Done when | Size |
|---|---|---|---|---|
| 15 | `book-identity` | The Android app's book id (the first 16 bytes of the file's SHA-256, in hex) instead of FNV-1a over parts of the file; the state's books, bookmarks and highlights moved to the new ids once | A book read in 0.11.0 opens where it was left, with its bookmarks and highlights; a file has the same id on Android and the desktop | 2 d |
| 16 | `folder-import` | Importing a whole folder, subfolders included, from the file dialog or a drop: copies into the Books folder, no duplicates, a count at the end | A folder of 300 books in subfolders imports without blocking the window; a second import adds nothing | 3 d |
| 17 | `library-metadata` | Series and number, genres, language and year from FB2, FB3 and EPUB; sorting and grouping (Recent, Author, Series, Title); search over series too; a book card with the annotation | 500 books group by author and by series, a series in its order | 1–1.5 w |
| 18 | `library-shelves` | The reader's own shelves and the automatic ones (Reading, Finished, Not started) | A book put on a shelf stays there after a restart | 1 w |
| 19 | `metadata-editing` | Editing title, authors, series and genre; kept by Forzac beside the reading state, book files untouched (`books-folder`) | An edited book sorts and groups by the edited fields; its file is byte for byte the same | 4–5 d |
| 20 | `books-folder-setting` | Choosing the Books folder in the settings (needed to keep the books in Dropbox, iCloud Drive or Syncthing), offering to move the books there | Switching to a folder in Dropbox keeps every book's position | 2–3 d |
| 21 | `send-to-kindle` | The book as MOBI 6 — what Calibre sends to the maintainer's Kindle (firmware 5.19, a mass storage disk), found on 2026-10-20 — with the cover, contents (an NCX index), notes, images and the reader's edits, written onto the Kindle connected by cable with the cover thumbnail its list needs. AZW3 was planned; an EPUB copied there does not show | A converted book opens on the maintainer's Kindle with its cover, contents and notes, as Calibre's copy does | 1.5 w |
| 22 | `sync-format` | A shared format in a synced folder (Dropbox, iCloud Drive, Google Drive, Syncthing; no server of our own): each device writes only its own file and reads the others', merged per item by time, deletions kept as tombstones; reading position, bookmarks, highlights, edited metadata, shelves. A position is (chapter, block, offset in code points) with the share read and a short text fragment: the desktop counts UTF-8 bytes and Android UTF-16 units, and the two parsers may number blocks differently. First between the Mac and the Pi 5 | A position, a bookmark and a highlight made on the Mac show on the Pi 5 through Dropbox, and nothing is lost when both changed offline | 1.5 w |

Then, in the Android repository: the same format, through Syncthing or a folder chosen with the system picker (Dropbox
and Google Drive keep no synced folder on Android; a direct Dropbox client only if that proves too weak), and the
reading log once the desktop has one.

## Test hardware

Plans count only on what is at hand:

- **Raspberry Pi 5** (8 GB, Raspberry Pi OS 13 trixie, labwc, Wayland) — the Linux reference: performance budgets,
  the GTK shell, the `.deb`. Measured through a headless 1920×1200 output at 120 Hz (`scripts/pi-headless.sh`) and
  looked at over VNC (`scripts/pi-vnc.sh`).
- **MacBook Pro 13" M1** (macOS 14, 60 Hz display, battery) — the AppKit shell, battery and idle measurements,
  smoothness at 60 Hz.
- **Containers** on the Mac (`ci/`) — Arch x86_64 (`makepkg`, install, `xdg-open` under headless sway), Fedora,
  Debian. They prove that packages build and install, not how the app looks or performs.

Not at hand: an Omarchy machine (x86_64 with Hyprland), a real 120 Hz display, a ProMotion Mac. Hyprland runs neither
in containers nor in a VM on this Mac (see the spike). Until Omarchy users confirm it, Omarchy support means "built and
installed in CI, not checked on hardware", and the README says so.

## Milestone 0.2 — a book you can read every day on Linux

FB2 comes before typesetting: notes, images, links and tables have to be in the model before they can be drawn.

| # | Change | What | Done when | Size |
|---|---|---|---|---|
| 1 | `project-setup` | Licence, public repository, CI (fmt, clippy, tests; Linux x86_64 and aarch64, macOS arm64 builds), contributor docs, `CLAUDE.md` | A pull request runs all checks; the repository can be published | 3–4 d |
| 2 | `fb2-complete` | Full FB2: notes bodies, images, links, tables, broken and truncated files, encodings; a corpus test | Every FB2 in the local corpus opens | 1 w |
| 3 | `typesetting-complete` | Images, footnote markers, links, tables, glyph fallback for characters the bundled fonts lack, background layout of the next chapters, layout cache | A real book's chapters render like the Android app; no `.notdef` boxes | 2–3 w |
| 4 | `epub` | EPUB 2/3 and FB3: spine, contents, notes, images; format detection by content | Every EPUB in the local corpus opens | 1.5–2 w |
| 5 | `reading-state` | Reading position per book, reopening where you left off, recent books, settings storage | Closing and reopening a book returns to the same line | 3–4 d |
| 6 | `reader-navigation` | Contents (`o`), footnotes, internal links with a way back, progress | A book can be read end to end with the keyboard only | 1–1.5 w |
| 7 | `page-turning` | The "Slide" mode specified, plus "Scroll" (continuous); switching with `m` | Both modes keep the reading position | 1 w |
| 8 | `reader-settings` | The preset settings of the Android app (theme, font, size, line spacing, alignment, margins) in a keyboard panel (`s`); light/dark from the system by default | Changing any setting keeps the reading position | 1 w |
| 9 | `book-search` | Search in the book (`/`, `n`, `N`), ё/е and case insensitive | A search jumps to the match and highlights it | 4–5 d |
| 10 | `linux-integration` | AUR package, `.desktop` and MIME, `xdg-open`, one process per user, icon, help and the support link | On the Pi 5 the `.deb` installs and `xdg-open book.fb2` opens Forzac; in the Arch x86_64 container `makepkg -si` installs and `xdg-mime query default` reports Forzac | 3–4 d |

## Milestone 0.3 — macOS

| # | Change | What | Done when | Size |
|---|---|---|---|---|
| 11 | `macos-app` | The AppKit shell catches up with 5–8 and the panels; opening from Finder; trackpad | Feature parity with the Linux shell on an Apple Silicon Mac | 2–3 w |
| 11a | `macos-release` | An ad-hoc signed `.app` in a DMG to download from forzac.app, with first-open instructions (no Developer ID, App Store or Homebrew) | On a fresh Mac user account the DMG from the site opens by following the instructions | 1–2 d |

## Milestone 0.4 — the same reader as on Android, tested on the Pi 5 and the Mac

Nothing is published (repository, AUR, `.deb`, DMG) before this milestone is done: the desktop app still differs from
the Android app in features and behaviour.

| # | Change | What | Done when | Size |
|---|---|---|---|---|
| 11b | `android-parity` | A list of every difference from the Android app in features and behaviour ([android-parity.md](android-parity.md)); each one decided as intended (keyboard first, no PDF, …) or a gap; the small gaps closed here; a by-hand pass on the Pi 5 and the Mac | No undecided difference is left; the by-hand pass finds nothing new | 2–3 d for the list, then 1.5–2 w |
| 11c | `viewer` | Images and tables in a full-window view: `Tab` stops at them, `Enter` or a click opens, `+`/`−` zoom, arrows pan, `Esc` closes | A schematic and a wide table can be read on the Pi 5 and the Mac | 4–5 d |

## Milestone 1.0

| # | Change | What | Done when | Size |
|---|---|---|---|---|
| 12 | `localization` | Interface in English and Russian | Every screen checked in both languages | 2–3 d |
| 12a | `two-page-spread` | Two pages side by side when the window is wider than tall and holds two text columns, as on Android tablets | A full-screen landscape window on the Pi 5 and the Mac shows two pages and keeps the reading position | 1 w |
| 13 | `bookmarks` | Bookmarks (`b`) and their list in the contents panel | A bookmark survives re-layout and restart | 3–4 d |
| 13a | `selection-quotes` | Selecting text with the mouse and from the keyboard (Vim's visual mode), copying, highlights (Android's quotes) and their list, export; asked for by the maintainer on 2026-10-17 | A highlight made from the keyboard on the Pi 5 survives re-layout and restart; copied text is clean | 1.5–2 w |
| 13b | `library` | The web reader's library instead of the recent books: covers (their own or generated), "Continue reading", search, removing, opening and dropping files; asked for by the maintainer on 2026-10-17 | 50 books open in the library in at most 200 ms on the Pi 5; checked by hand in both languages on the Pi 5 and the Mac | 1 w |
| 13c | `reader-menu` | The web reader's reading menu one to one (back to the books, section, contents; a slider over the whole book with chapter ticks, "Page N of M", minutes left in the chapter, "Aa"); the whole book typeset in the background for page numbers in the foot and the lists. Asked for by the maintainer on 2026-10-17, reversing `android-parity` R1 and R3 | The menu checked by hand on the Pi 5 and the Mac in both languages; page turns miss no more frames while the book is counted | 1 w |
| 13d | `help-card` | The help as a card in the web reader's style: keys as key caps in sections, two columns in a wide window, the support line with its link; the core keeps it as data, ready for the web reader; asked for by the maintainer on 2026-10-17 | Screenshots in both languages on the Mac and GTK; `?`, `Esc`, a click beside it close it, other keys do nothing | 2 d |
| 14 | `release-1-0` | Performance budgets checked on the Pi 5 and the Mac; x86_64 packages checked in containers; release notes; packages published | Budgets met on the Pi 5 and the Mac | 3–4 d |

Total: about 18–22 weeks.

## After 1.0

- `accessibility`: the page's text for screen readers (Orca, VoiceOver), as TalkBack reads it on Android.

## From the UX review

[ux-review.md](ux-review.md) (2026-10-20) compared Apple Books, Kindle for Mac, Foliate, KOReader, Readest, Thorium and
Bookmate. Chosen by the maintainer on 2026-10-20:

| # | Change | What | Done when | Size |
|---|---|---|---|---|
| 1 | `macos-reading-bar` | On macOS the page under a transparent title bar; a bar with the window buttons, back, Contents, Aa, Search, Bookmark around the page's own running head, coming with the pointer and fading 2 s after it stops, the pointer hidden with it; the same in full screen; Linux unchanged (review W1, W2, O1, B1) | Screenshots on the M1; no missed frames while the bar fades; no CPU while idle | 3–4 d |

Done and archived 2026-10-20: `macos-reading-bar`, and from the maintainer's look at it `spread-heads` (a spread's left
head is the book's title, the right one the chapter's; the ribbon hangs over the right page, never in the gutter).

The review's library items — collections with automatic ones, sorting, editing metadata without touching the file
(L1–L3) — are rows 17–19 above (`library-metadata`, `library-shelves`, `metadata-editing`).

Candidates the maintainer has not chosen (review ids): Forward in the jump history (P1); a page number in the search
field and searching the selection (P2, C3); following the system's "Reduce motion" (P3); `⌥` arrows for chapters on
macOS (P4); time left from the reader's own speed (W3, W4); Look Up on macOS (D1); notes on highlights and a Markdown
file of them (H1, H2).

## Not planned for 1.0

Confirmed with the maintainer in `android-parity` (2026-10-16). Text selection and quotes left the list on 2026-10-17,
when the maintainer asked for them (`selection-quotes`), and so did the library screen (`library`); importing books
left it on 2026-10-19 (`books-folder`).

PDF; dictionaries; formulas (MathML); fixed-layout EPUB; Flatpak (AUR and
the `.deb` first); a Developer ID, notarisation, Homebrew and the App Store; following the Omarchy theme colours (a possible later change).

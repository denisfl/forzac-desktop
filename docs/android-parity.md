# Parity with Forzac for Android

Every reader feature and behaviour of the Android app next to how the desktop app does it, for the
[`android-parity`](../openspec/changes/archive/2026-10-16-android-parity/proposal.md) change (milestone 0.4). Compared on 2026-10-16:
Android at `b6b3664`, desktop at `d55130a`.

**Status:** **same** — the reader sees the same thing (the means may differ); **intended** — a difference the plans
already allow: the keyboard instead of touch, no PDF, no "Curl" and "Cover", an item under "Not planned for 1.0" in
[roadmap.md](roadmap.md); **gap** — the desktop app lacks or does differently something it should do; **unclear** — a
difference nobody has decided yet. Every gap and unclear row was decided by the maintainer; the decisions are at the
end, and the by-hand pass on the Pi 5 and the Mac is recorded after them.

**References.** Android paths are in the Android repository: a bare `Name.kt` is
`app/src/main/java/app/truenrth/forzac/Name.kt`; `typesetting/`, `model/`, `fb2/`, `epub/`, `data/` are the core
modules (`core/typesetting/src/main/kotlin/app/truenrth/forzac/typesetting/…` and so on); "A spec x" is its
`openspec/specs/x/spec.md` (or the active change of that name). Desktop paths are in this repository; "spec x" is
`openspec/specs/x/spec.md`. Symbols in parentheses.

## 1. Opening books and formats

| # | Android | Desktop | Status |
|---|---|---|---|
| F1 | FB2 and FB2.ZIP: encodings, HTML entities, parts opened into chapters, notes, images, cover, tables, truncated files keep what was read (`fb2/Fb2Reader.kt`, A spec fb2-parsing) | The same rules (`core/src/fb2.rs`, spec fb2-parsing); also a bold first row as table header and a 256 MB unpack limit | same |
| F2 | FB3 through the same reader (`fb2/…/fb3/Fb3Parser.kt`, A change fb3 — waits for a live LitRes file) | `core/src/fb3.rs`, spec fb3-parsing | same |
| F3 | EPUB 2/3: package, spine, split chapters joined, nav/NCX, notes, links, DRM rejected, ruby dropped, images in the spine, the cover page skipped (`epub/EpubReader.kt`, `epub/XhtmlReader.kt`, A spec epub-parsing) | The same rules (`core/src/epub/`, spec epub-parsing); also explicit CJK line breaking, which Android gets from `LineBreaker` | same |
| F4 | Format decided by content (`data/BookParser.kt`) | `core/src/lib.rs` (`parse_format`) | same |
| F5 | PDF as pages with text layer, search, quotes (`PdfBook.kt`, `PdfReaderScreen.kt`) | None | intended (no PDF) |
| F6 | Fixed-layout EPUB of picture pages: fit to screen, manga right to left, spreads (`FixedLayoutScreen.kt`, A spec fixed-layout) | Read as ordinary content; a book of images only fails as "no text" (`core/src/epub/mod.rs`) | intended (Not planned for 1.0, confirmed) |
| F7 | MathML formulas typeset by its own engine, inline and display, shrunk to the column, viewer (`model/Math.kt`, `typesetting/MathLayout.kt`, `FormulaViewer.kt`, A spec math) | MathML read as plain text, annotations dropped (`core/src/epub/xhtml.rs`, skip list) | intended (Not planned for 1.0, confirmed) |
| F8 | A book is imported into the library (copied, id = content hash) from the file picker, "Open with" and "Share", then opened (`MainActivity.kt` `handleIntent`, `LibraryViewModel.kt`, A spec library) | Opened in place: command line, `xdg-open` and file managers (`gtk/src/main.rs`, `HANDLES_OPEN`), Finder, the Dock, `⌘O` (`macos/main.swift`, `AppDelegate.open`), the library's "Open a book" and a file dropped on it (`gtk/src/library.rs`, `macos/Library.swift`); the library lists the books read, without copying them | intended (no import) |

## 2. Typesetting

| # | Android | Desktop | Status |
|---|---|---|---|
| T1 | `StaticLayout`, high-quality breaking, inter-word justification, last line natural (`typesetting/BlockLayouts.kt` `build`) | Knuth–Plass over HarfRust-shaped words (`core/src/knuth_plass.rs`, `core/src/layout/mod.rs`), [ADR 0001](adr/0001-stack.md) | same |
| T2 | Hyphenation for any language Android has patterns for, by the book's locale (`BlockLayouts.kt` `textLocale`) | Russian and English patterns only; any other language is not hyphenated (`core/src/hyphen.rs` `for_language`) | unclear |
| T3 | A book without a language: guessed from its text, Cyrillic → `ru`, else `en` (`model/BookLanguage.kt`) | Taken as Russian (`core/src/hyphen.rs`, `unwrap_or("ru")`); detection is planned in `localization` | gap |
| T4 | Paragraph kinds: indent 1.5 em, citation 0.92 wide, epigraph 0.7 wide italic 0.92, author right, verse 0.88 unhyphenated, subtitle bold centred kept with next (`BlockLayouts.kt` `paragraph`) | The same geometry (`core/src/layout/mod.rs` `Typesetter::layout`) | same |
| T5 | Headings bold centred, 1.35 / 1.15, unhyphenated, kept with next (`BlockLayouts.kt` `heading`) | The same | same |
| T6 | Preformatted text (`pre`, code) in a monospace face at 0.85 (`BlockLayouts.kt`, `Kind.Code`) | At 0.85 in the book face, line breaks kept (`core/src/layout/mod.rs`, `Kind::Code`) | gap |
| T7 | Inline code in monospace at 0.9; strikethrough drawn (`BlockLayouts.kt` `spansOf`) | Both parsed, drawn as plain text (`core/src/layout/mod.rs` `segments`) | gap |
| T8 | Superscript, subscript and note markers at 0.7 with a rise, line height unchanged | The same (`segments`) | same |
| T9 | Links in the theme's link ochre, no underline | The same colours (`gtk/src/page_view.rs` `THEMES`, `Ink::Link`) | same |
| T10 | No lone first or last line, heading kept with two lines, chapter on a new page lowered two lines, no empty line at a page top, images and table rows never split, header row with the first data row (`typesetting/Paginator.kt`) | A port of the same paginator (`core/src/layout/page.rs` `paginate`) | same |
| T11 | Images one dp per pixel, scaled to the column and page, centred, half a line around, broken ones skipped, decoded only near the shown page (`BlockLayouts.kt` `image`, `PagePictures.kt`) | The same, one logical pixel per pixel; neighbours' images decoded on a worker (`core/src/layout/mod.rs` `image`, `page_view.rs` `prefetch`) | same |
| T12 | Tables: widths by content, 0.9 then 0.8 size, rules heavy/medium/light, bold header (`BlockLayouts.kt` `geometry`, A spec tables) | The same (`core/src/layout/table.rs`, spec typesetting) | same |
| T13 | Missing glyphs from system fonts (`BookFonts.kt`, `CustomFallbackBuilder`) | System fallback fonts, drawn from the same file (`core/src/fonts.rs`, `gtk/src/fonts.rs`) | same |
| T14 | Typographic normalisation: no-break spaces, em dashes (`model/TypographyNormalizer.kt`) | A port (`core/src/typography.rs`) | same |

## 3. Page turning and modes

| # | Android | Desktop | Status |
|---|---|---|---|
| M1 | "Slide", "Cover", "Curl" (Android 13+), "Scroll" (`PageTurn.kt`) | "Slide" and "Scroll" (`core/src/settings.rs` `Turning`) | intended (no Curl and Cover) |
| M2 | The page follows the finger, a fling turns, taps in the left/right third turn, the middle opens the menu (`ReaderScreen.kt` `readerGestures`) | Keys (`j l → ↓ Space PgDn Enter` / `k h ← ↑ Shift+Space PgUp Backspace`), a 200 ms ease-out slide; a wheel step turns a page, a touchpad swipe at most one (`page_view.rs` `turn`, `macos/PageView.swift` `keyDown`, `scrollWheel`). A click on the page turns nothing | unclear (click to turn) |
| M3 | "Scroll": one column of the same typesetting, chapters apart, everything works in it (`ReaderScreen.kt` `FlowView`) | One column of tiles, a gap between chapters; line, screen and half-screen keys, wheel by three lines, kinetic touchpad (`gtk/src/scroll.rs`, `macos/PageView.swift`) | same |
| M4 | Switching modes keeps the line (A spec page-turning) | `m` keeps the top line (spec page-turning) | same |
| M5 | Asks for the highest refresh rate while moving, nothing drawn at rest (`HighRefreshRate`) | Frame clock / `CADisplayLink`, nothing drawn at rest | same |
| M6 | Tablet in landscape (short side ≥ 600 dp, wider than tall): two pages side by side (`ReaderScreen.kt` `spread`, `ReaderSurface.kt` `PagedSurface.perScreen`) | One column at most the measure wide, centred, however wide the window (`core/src/settings.rs` `page_style`) | unclear |
| M7 | After the last page an end page: the book closes into its cover, "Finished", days read, highlights and bookmarks, "Share highlights", "Back to library"; the book is marked 100 % (`BookOpening.kt` `FinishPage`, A spec reader-ui "Финальная страница") | Turning past the last page does nothing; the last page shows the share before it, never 100 % (`page_view.rs` `turn`, `core/src/state.rs` `progress`) | unclear |

## 4. Contents, notes and links

| # | Android | Desktop | Status |
|---|---|---|---|
| N1 | Contents full screen: tabs Contents / Bookmarks / Highlights, a "Search in book" row, the book (cover, title, author · N% read), entries with nesting, the current one bold with a dot, page numbers (`TocScreen.kt`) | A side panel (`o`): "Contents", a filter field, the book card, entries with nesting, current bold with a dot, positions in percent; `j`/`k`, `Enter`, `/` (`gtk/src/panels.rs` `Contents`, `macos/Panels.swift` `ContentsPanel`) | intended (keyboard); tabs follow Q1, Q2; numbers follow R1 |
| N2 | A book without contents lists every chapter, an untitled one as "Chapter N" (`ReaderScreen.kt`, `entries`) | Only titled chapters are listed; untitled ones cannot be reached from the contents (`core/src/model.rs` `Book::contents`) | gap |
| N3 | A book without a cover gets a generated one (marble or cloth) in the contents (`Covers.kt` `GeneratedCover`) | A plain dark box with the title (`panels.rs` `cover`, `Panels.swift` `cover(of:)`) | unclear |
| N4 | A note in a bottom sheet: grabber, "NOTE n", text with images and tables, scrolls, closes; the page stays (`NoteSheet.kt`) | The same sheet, the note typeset by the core like the book (`panels.rs` `note`, `Panels.swift` `NoteSheet`, `core/src/layout/page.rs` `layout_note`) | same |
| N5 | A tap on a marker or link (16 dp slop) wins over the tap zones | `Tab`/`Shift+Tab` focus, `Enter` opens; a click opens (4 px slack) (`page_view.rs` `focus_next`, `click`) | intended (keyboard) |
| N6 | A link to a short target (≤ 3 blocks and 600 characters: a figure, a table, a paragraph) opens in the note sheet, the page stays (`ReaderScreen.kt` `openRef`, `model/LinkPreview.kt`) | Every link to a place jumps there (`page_view.rs` `open`) | gap |
| N7 | External links open in the default browser or mail app (`openExternal`) | The same (`gtk::UriLauncher`, `NSWorkspace.open`) | same |
| N8 | After a big jump (link, contents, bookmark, highlight, search, slider beyond the next page) a pill "← Back · p. N"; it returns to where the first jump of a series started, vanishes after the reader turns two pages on; Back returns (`ReaderViewModel.kt` `leave`/`returnPoint`, `ReaderScreen.kt` `ReturnPill`) | `Ctrl+O` pops a stack of up to 50 places pushed by links, contents and search; nothing on screen says it exists; `[ ] g G` do not push (`page_view.rs` `go_to`, `back`) | unclear |
| N9 | — (the menu slider) | Chapter keys `]` `[`, chapter start/end `g` `G` `Home` `End` | intended (keyboard) |

## 5. Search

| # | Android | Desktop | Status |
|---|---|---|---|
| S1 | Case and ё/е ignored, no-break space = space, soft hyphen does not interrupt, ≥ 2 characters, notes not searched (`model/Search.kt`) | The same, and a space matches any white space (`core/src/search.rs`) | same |
| S2 | Table cells are searched and the match is highlighted in the cell (`Search.kt`, A spec tables) | Table rows are not searched (`core/src/search.rs`) | gap |
| S3 | Full screen over the contents: count, snippets (40/80, match bold), chapter · page; a tap jumps, highlights and shows the Back pill; the highlight goes on the next tap; the query is kept (`SearchScreen.kt`) | A side panel (`/`, `⌘F`): the same count and snippets, section · percent; `Enter` jumps and highlights; `n`/`N` next and previous; `Esc` clears the highlight; the query is kept (`panels.rs` `SearchPanel`, `Panels.swift` `SearchPanel`) | intended (keyboard); numbers follow R1 |

## 6. Where the reader is

| # | Android | Desktop | Status |
|---|---|---|---|
| R1 | The whole book is typeset in the background; then the foot shows "N of M" for the whole book, and the menu, contents, search and bookmarks show page numbers (`ReaderViewModel.kt` `layout`, `ReaderScreen.kt` `PagedView`) | Only the current and next chapters are typeset; the foot shows "4 / 22 · 37%" — page in chapter / chapter's pages · share of the book; lists show percent (`core/src/layout/page.rs` `render`) | unclear |
| R2 | No running head: the page has the text and the page number only (A spec reader-ui "Экран чтения без интерфейса") | The chapter title as a running head on every page but the chapter's first (`page.rs` `render`, spec reader-navigation) | unclear |
| R3 | The menu (middle tap): back, chapter title, author · book, contents; slider with chapter ticks; "Page N of M"; "N min left in chapter" (`ReaderMenu.kt`) | No menu; the window title is the book's title; chapter keys and the contents instead; no time estimate | unclear |
| R4 | "N% read" in the contents and the library | "N% read" in the contents, the foot and the recent books | same |

## 7. Settings and themes

| # | Android | Desktop | Status |
|---|---|---|---|
| C1 | Seven settings in the "Aa" sheet: theme, size, alignment, font, spacing, margins, page turn (`SettingsSheet.kt`) | The same seven in the `s` sheet with keys, plus direct keys `+ − 0 t T m` and the Mac menus (`gtk/src/settings_sheet.rs`, `Panels.swift` `SettingsSheet`) | same |
| C2 | Six themes: Light, Sepia, Dark, Dawn, Wave, Forest (`ReaderColors.kt`) | The same palettes, colour for colour (`page_view.rs` `THEMES`) | same |
| C3 | Light until the reader picks a theme (`ReaderSettings.kt`) | Follows the system's light or dark until the reader picks one (`core/src/settings.rs` `theme_index`, `gtk/src/appearance.rs`) | unclear |
| C4 | A theme change cross-fades in 220 ms (`ReaderScreen.kt` `ThemeCrossfade`) | Instant (`page_view.rs` `show_theme`) | gap |
| C5 | Fonts: Noto Serif (default; Android's system serif), Literata, PT Serif, Source Serif, PT Sans (`BookFonts.kt`) | PT Serif (default), Literata, Source Serif, PT Sans, all bundled (`core/src/fonts.rs` `Family`) | unclear |
| C6 | Sizes 15, 16, 17, 18, 20, 22, 24 sp, default 18 | Sizes 16, 18, 20, 22, 24, 27, 30 px, default 20 (`core/src/settings.rs` `TEXT_SIZES`) | unclear |
| C7 | Spacing 1.45 / 1.62 / 1.85, alignment justified / left | The same | same |
| C8 | Margins 18 / 28 / 40 dp, the column at most 34 em (tablet-layout) | Margins as the widest column, 38 / 34 / 30 em, and the least side margin, 24 / 48 / 72 px (`core/src/settings.rs` `Margins::column`) | unclear |
| C9 | Settings global and remembered; an unknown value falls back to the default | The same (`Settings::load`/`save`, spec reader-settings) | same |
| C10 | The theme colours the page, menu, contents, notes, settings, system bar icons and the library (`MainActivity.kt`) | The page, the panels, the Mac window's appearance and the library (`gtk/src/library.rs` `set_theme`, `macos/Library.swift`) | same |

## 8. Reading position and recent books

| # | Android | Desktop | Status |
|---|---|---|---|
| P1 | The position is a `Locator`, restored on reopening, kept through re-layout and rotation (A spec reading-position) | The same, kept through resizing and settings (spec reading-state, spec typesetting) | same |
| P2 | Book identity: hash of the imported file | Size + hash of the first and last 64 KiB, path as a fallback (`core/src/state.rs` `book_id`) | same |
| P3 | Written on every page shown (`ReaderViewModel.kt` `positions`) | On opening, 1 s after the last move, on closing (`gtk/src/main.rs` `remember_position`, `main.swift` `savePosition`) | same |
| P4 | Library: "Continue" card, grid of covers with progress, generated covers, add, search, delete, empty state (`LibraryScreen.kt`, A spec library) | The library (`library`, 2026-10-17): the web reader's shelf of covers with Android's "Continue reading" card, the share read, generated covers, search, removing with a question, the web's empty state and dropping files; books open in place; the card has no chapter title or time left (the state keeps neither) (`gtk/src/library.rs`, `macos/Library.swift`) | same (NP3 reversed) |
| P5 | Back from a book returns to the library | Both shells show the library after the last book window closes (`gtk/src/main.rs` `connect_window_removed`, `main.swift` `onClose`) | same |
| P6 | A "Continue" shortcut on the app icon (`ContinueShortcut.kt`) | Open Recent on the Mac; nothing on Linux | intended (platform) |

## 9. Errors

| # | Android | Desktop | Status |
|---|---|---|---|
| E1 | "Couldn't open “name”: reason" in the library, reasons: unsupported format, damaged, DRM-protected, no text, file unavailable (`LibraryViewModel.kt` `errorText`, `strings.xml`) | GTK: a line on stderr and no window — opened from a file manager, nothing happens (`gtk/src/main.rs` `open_window`). macOS: an alert "Not a readable book" with the file name, no reason (`main.swift` `open`). The core knows not a book / no text / protected; an unreadable file counts as "not a book" (`core/src/lib.rs` `open`) | gap |
| E2 | Damaged files keep what could be read | The same (spec fb2-parsing, spec epub-parsing) | same |

## 10. Images and tables

| # | Android | Desktop | Status |
|---|---|---|---|
| V1 | A tap on an image at least a third of the column wide opens it full screen: pinch to ×5, double tap ×2.5, pan, swipe down or Back closes (`ImageViewer.kt`, A change image-viewer) | The same image, `Tab` and `Enter` or a click, fitted to the window: `+`/`−`/`0`, a pinch or `Ctrl`+wheel to ×5, arrows, wheel, touchpad or drag pan; a click, `Esc` or `q` closes (`gtk/src/viewer.rs`, `macos/Viewer.swift`; change `viewer`) | same (keys for the double tap and the swipe) |
| V2 | A tap on a table opens it full screen: columns by content, pan both ways, pinch 50–300 % (`TableViewer.kt`, A spec tables) | The same: columns by content up to 20 em at the text size, 50–300 % in 5 % steps typeset again, pan both ways; markers and links in cells work too (`core/src/layout/table.rs` `layout_table`; change `viewer`) | same |

## 11. Bookmarks and highlights

| # | Android | Desktop | Status |
|---|---|---|---|
| Q1 | Bookmarks: a tap in the top right corner, a ribbon with cloth physics, a list tab in the contents (A spec bookmarks) | `b` (or a click in the top right corner) sets or removes; the same ribbon drops in and slides with its page, without the swing; a "Bookmarks" tab in the contents (`B`), `d` removes (change `bookmarks`) | same (keys; no swing) |
| Q2 | Selection by long press, highlights, Copy, Share, export, a Highlights tab (`TextSelection.kt`, A spec quotes) | A drag by words, a double or triple click, `Shift`+click, handles; from the keyboard Vim's visual mode (`v`, `h l j k`, `v`, `o`); the bar "Highlight · Copy" (+ "Share" on macOS); highlights under the text with the stroke; a "Highlights" tab, copying all in Android's export format (change `selection-quotes`) | same (keyboard; no Share on Linux) |

## 12. Screen, window and the rest

| # | Android | Desktop | Status |
|---|---|---|---|
| X1 | Opening a book plays the cover turning to the marbled endpaper and the title page, hiding the load (`BookOpening.kt`, A spec reader-ui "Жест открытия книги") | The first page appears | unclear |
| X2 | The screen stays on while reading, for 10 minutes after the last touch (`ReaderScreen.kt` `KeepScreenOn`) | The system blanks the screen and sleeps the display as usual | gap |
| X3 | Always immersive; system bars come with the menu (`SystemBars`) | A window; `f`/`F11` (`⌃⌘F`) full screen | intended (platform) |
| X4 | TalkBack reads the page and has next/previous page, bookmark and menu actions (`ReaderScreen.kt` `semantics`) | The page is drawn without accessible text (`page_view.rs`, `PageView.swift`) | unclear |
| X5 | Interface in English and Russian (A spec localization) | English and Russian by the system's preferred languages, one catalogue in the core in the Android app's words (`core/src/i18n.rs`; change `localization`) | same |
| X6 | — | `?` shows the keys and the single support line (`gtk/src/main.rs` `HELP`, `main.swift` `helpText`) | intended (desktop only) |

## 13. Android work not shipped yet

| # | Android | Desktop | Status |
|---|---|---|---|
| L1 | Reading log by day, finish dates, history that survives deletion (A change reading-log, tasks open, nothing in the app yet) | None | unclear |

## 14. Between the two desktop shells

Found on the way; the by-hand pass would hit them.

| # | GTK | macOS | Status |
|---|---|---|---|
| D1 | Opening a book that is already open opens a second window (`gtk/src/main.rs` `connect_open`) | Brings its window forward (`main.swift` `open`) | gap |
| D2 | The library: arrows and `h` `j` `k` `l`, `Enter`, `Delete`/`d`, `/`, `q`/`Esc` | The same keys, by key code (`main.swift` `LibraryWindow.key`) | same |

## 15. Not planned for 1.0 — to confirm or reopen

From [roadmap.md](roadmap.md):

| # | Item | Rows |
|---|---|---|
| NP1 | PDF | F5 |
| NP2 | Text selection and quotes (highlights, copy, share) — reopened 2026-10-17 | Q2 |
| NP3 | A library screen with import and covers; recent books are enough — reversed 2026-10-17: the library without import | F8, P4, N3, X1 |
| NP4 | Dictionaries | (Android: draft change `dictionary`, not decided) |
| NP5 | Formulas (MathML) | F7 |
| NP6 | Fixed-layout EPUB | F6 |
| NP7 | Flatpak (AUR and the `.deb` first) | — |
| NP8 | A Developer ID, notarisation, Homebrew and the App Store | — |
| NP9 | Following the Omarchy theme colours | — |

## Decisions

Every gap, unclear and "Not planned" row with its proposal. The maintainer accepted every proposal as written on
2026-10-16. "Lands in" says where the work goes: this change (`android-parity`), its own change, or nowhere.

| # | Decision | Lands in |
|---|---|---|
| T2 | Keep Russian and English patterns for 1.0; other languages stay unhyphenated | — |
| T3 | Detect the language from the text as Android does, in this change (moved from `localization`) | android-parity |
| T6, T7 | Bundle a monospace OFL font with Cyrillic (PT Mono) for code blocks and inline code; draw strikethrough | android-parity |
| M2 | A click in the left or right third of the page turns it; the middle does nothing | android-parity |
| M6 | Two pages side by side when the window is wider than tall and holds two columns: its own change after 0.4 | `two-page-spread` (1.0) |
| M7 | A plain end page after the last page ("Finished", title and author), marking the book 100 %; no animation | android-parity |
| N2 | List untitled chapters as "Chapter N" | android-parity |
| N3 | Keep the plain cover; generated covers belong to the library | — |
| N6 | Short link targets in the note sheet, by the Android rule, in the core | android-parity |
| N8 | Keep the `Ctrl+O` stack and show Android's "← Back · Ctrl+O" pill after a jump, gone after two pages on | android-parity |
| S2 | Search table rows and highlight the match in the cell | android-parity |
| R1 | Keep chapter pagination and "4 / 22 · 37%": no whole-book layout on every resize | — |
| R2 | Keep the running head: without a menu it is the only place the chapter title shows | — |
| R3 | No menu and no time estimate on desktop | — |
| C3 | Keep following the system's light or dark | — |
| C4 | A 200 ms cross-fade on theme change | android-parity |
| C5 | Keep the four bundled fonts with PT Serif first (Noto Serif is Android's system font, not a choice made for the book) | — |
| C6 | Keep the desktop sizes (a screen read from further away) | — |
| C8 | Keep the desktop margins (any window width) | — |
| C10 | The recent-books window in the reader's theme | android-parity |
| P5 | GTK shows the recent books after the last book closes, as macOS does | android-parity |
| E1 | A message in both shells with Android's wording and reason; "file unavailable" apart from "not a book" | android-parity |
| V1, V2 | One viewer for images and tables (`Tab` stops at them, `Enter` or a click opens, `+`/`−` zoom, arrows pan, `Esc` closes): its own change in 0.4 | `viewer` (0.4) |
| Q1 | Bookmarks stay in `bookmarks` (1.0) | `bookmarks` (1.0) |
| X1 | No opening animation on desktop | — |
| X2 | Keep the screen on while reading, for 10 minutes after the last key, turn or scroll | android-parity |
| X4 | Screen-reader text of the page: its own change after 1.0 | `accessibility` (after 1.0) |
| X5 | Localisation stays in `localization` (1.0) | `localization` (1.0) |
| L1 | Out of scope until the Android app ships it | — |
| D1, D2 | Make the shells agree: GTK brings an open book forward; the Mac recent list takes `j`/`k`/`Esc` | android-parity |
| NP1–NP9 | Confirm all | — (roadmap unchanged) |
| NP3, P4 | Reversed by the maintainer on 2026-10-17, after looking at the Mac build next to the Android app and the web reader ("no library, no reading menu, no visible chapter navigation"): the desktop gets the web reader's library 1:1 with Android's card, covers and search, keeping its keys; books still open in place | `library` (1.0) |
| NP2, Q2 | Reopened by the maintainer on 2026-10-17: selection and highlights as on Android; the keyboard selects as Vim's visual mode, a drag snaps to words, no "Share" on Linux | `selection-quotes` (1.0) |

## By-hand pass

Done at the end of `android-parity` (its tasks, groups 5 and 6) on the Raspberry Pi 5 and the M1 MacBook Pro, on
2026-10-16, with the build of this change. The Pi's desktop (labwc, Raspberry Pi OS 13, GTK 4.18) was driven over SSH
with `wtype` and watched with `grim`; timings came from `scripts/pi-headless.sh` at 1920×1200, 120 Hz. The Mac was
driven with mouse clicks and arrow keys (synthetic letter keys did not reach the app, so letter shortcuts were reached
through the menus).

**Timings on the Pi 5.** First page after opening "Колыбель для кошки" (warm): 151–158 ms with the default, `ngl` and
`gl` renderers (Vulkan 165–170 ms after a first start of 2.35 s, its shader cache). Page turns (20 per run): 0 missed
frames at 122 Hz with the default, `ngl` and `gl` renderers, 2 with Vulkan. Scrolling by a screen (20 per run): 0 missed
frames in most runs, now and then 1–4 frames late at the first or second frame of a movement; the build before this
change does the same (8 paired runs: 2 late frames before, 5 after — within the runs' spread). Idle: 0 CPU ticks in
10 s.

**Checked and passing, both shells:** an English book without a language is hyphenated in English (T3); code in PT Mono
and struck-out text drawn (T6, T7); the end page after the last page in "Slide" and the end screen in "Scroll", saved
as 100 % and back to the text with `←`/`k` (M7); a link to a short target opens in the note sheet (N6); the "← Back"
pill after a contents jump, gone after two pages (N8); a non-book file gives "Couldn't open “photo.zip”: unsupported
format — FB2, FB3 or EPUB needed", then the recent books (E1); after the last book window the recent books, in the
reader's theme (P5, C10); the screen kept on while reading and released 10 minutes after the last input — the Wayland
idle inhibitor on the Pi, the `PreventUserIdleDisplaySleep` assertion on the Mac (X2). The Pi: opening an open book
brings its window forward (D1). The Mac: a click in the left third turns back (M2); the recent books move with the
arrows and close with `Esc` (D2); the theme changes from the View menu (C4: the fade is too short to see on a
screenshot); the contents open from the menu and jump.

**Found and fixed in this pass:**

- macOS quit when its last book window closed, before the recent books could show (P5): it now stays, and closing the
  recent books quits.
- macOS showed no message for a file that came with the launch (Finder, the Dock): the alert now waits for the launch.
- The recent books' selection on macOS and the list background on GTK kept the system colours: both in the theme now.
- GTK did not show the recent books after the last book (P5), and opening a book again after closing it showed nothing
  and quit (D1): a closed book window stays alive while closures hold it, so it never gets `destroy`. The recent books
  now follow the application's `window-removed`, open windows are those still in the application, and the screen
  inhibitor is released on `close-request`.

**Not checked by hand in this pass:** the Pi's side-third clicks (M2; no pointer injection over SSH; the same code path
as the Mac's), the rows marked "same" that earlier changes had checked by hand (settings panel, search panel and
`n`/`N`, notes and `Tab` focus, trackpad, Finder and the Dock), and the cross-fade by eye. The maintainer accepted the pass as it is
(2026-10-16): those rows were checked by hand in the changes that made them.

| Machine | Date | Build | Result |
|---|---|---|---|
| Raspberry Pi 5 | 2026-10-16 | `android-parity`, GTK shell | Passing after the GTK fixes above; M2 not checked |
| M1 MacBook Pro | 2026-10-16 | `android-parity`, AppKit shell | Passing after the macOS fixes above |

### The view of images and tables (`viewer`, V1, V2)

Checked on 2026-10-16 with the build of the change. The Pi's desktop was driven with `wtype` and watched with `grim`;
the Mac build was driven with the shell's `--panel image:ID|table:C.B --keys …` and `--png` (computer control of the Mac
was not available), so its keys went through `NSWindow.sendEvent` to the real handlers.

- A schematic: "LLM Engineer's Handbook", figure 1.6 (825×829). On both: `Tab` outlines it, `Return` opens it fitted
  to the window, `+` four times and `l` twice show its middle right part at ×2.44, `0` fits it again, `Esc` returns to
  the same page with the outline still on it. A 120-pixel ornament is not a stop.
- A wide table: the same book's dataframe table (7 columns, 2170 px at the text size; on the page set at 0.8 and
  unreadable). On both: it opens at the text size from its top left corner, `l` pans to the right columns, `End` to the
  bottom, `−` to 65 %, `Tab` walks the links of the last column and pans each into the window.
- A table with a note marker and a link in a cell (a test book): `Tab` on the page stops at the table before the
  marker; in the view the note opens in the sheet over it, `Esc` gives the keys back to the view, the link closes the
  view and jumps, `Ctrl+O` returns. On the Pi also in "Scroll"; on the Mac also in the Dark and Wave themes.
- Found and fixed: links inside table cells were never registered by the FB2 and EPUB readers, so they were dead on the
  page too.
- Cost on the Pi 5: 0 CPU ticks in 10 s with a table view open; `scripts/pi-headless.sh` idle 0 ticks. Page turns at
  123 Hz, 12 runs of 20 turns alternating the build before the change and this one (default renderer): 12 and 11 late
  first frames of a turn, the same spread (single runs 0–3 for both). Typesetting a table for the view: a 101-row table
  in 10 ms on the Pi (the corpus test with the two books above and a test book), the corpus's slowest in 1.5 ms on the
  Mac.
- Not checked by hand: the mouse, the wheel, the touchpad and the pinch in the view on both machines (no pointer
  injection on the Pi, no computer control of the Mac), and a theme change while the view is open.

### Bookmarks (`bookmarks`, Q1)

Checked on 2026-10-17 with the build of the change. The Pi was driven with `wtype` in a second, headless labwc (the
desktop session's file-search field keeps the keyboard focus there) and watched with `grim`; the Mac build was driven
with `--panel … --keys …` and `--png` (computer control of the Mac was not used), so its keys went through
`NSWindow.sendEvent` to the real handlers.

- On both: `b` drops the ribbon onto the page (bordeaux in the light themes, lighter in Dark and Wave), `b` again takes
  it up; turning to a marked page brings the ribbon in with the page, turning away takes it out with it; in "Scroll" the
  ribbon hangs while the marked line is on screen and goes when it leaves (the Mac: `G` to the chapter's end and `g`
  back). The contents panel's tabs: `→`/`l` and `Tab` switch, the filter field shows on "Contents" only; the list shows
  the section, the percent and two lines of text; `Enter` goes to the page with the "← Back" pill; `d` removes, and the
  last removal shows "No bookmarks yet" with the hint. The Mac: `B` opens the tab and closes the panel from it.
- The Pi: a bookmark set in "Scroll" marks the page that holds its line after switching to "Slide" and after two
  larger text sizes; bookmarks are in the state file and in the list after a restart.
- Found and fixed: the drop started 12 px above the window (the ribbon's tuck), so its first frames showed nothing new
  and each drop was a frame late on the Pi; it now starts just out of the window. On the Mac a ribbon dropping when the
  reader moved away stayed on screen until the drop ended.
- Cost on the Pi 5 (`scripts/pi-headless.sh`, 1920×1200 at 123 Hz): page turns 0 missed frames with the default and
  `ngl` renderers, 1 with `vulkan` and `gl`; idle 0 CPU ticks in 10 s, also with a ribbon on the page. Six paired runs
  with a bookmark at every paragraph (`--marked`) and without: 2 and 1 late frames in 120 page turns, 1 and 4 in 120
  scrolls by a screen. 20 drops and rises (`--toggles 20`), three runs per mode: 0–1 late frames in "Slide", 1–2 in
  "Scroll", each the first frame after a pause, as page turns have. 200 bookmarks listed in at most 1.1 ms and saved in
  at most 0.8 ms (the corpus test on the Pi's books).
- Not checked by hand: `B` on the Pi (`wtype` cannot send a shifted key there; `T` does not arrive either), the corner
  click on both (no pointer injection), Go → Bookmark `⌘D` and View → Bookmarks on the Mac (menus are not reachable
  through `--keys`). The maintainer accepted the pass as it is (2026-10-17).

### Selection and highlights (`selection-quotes`, Q2)

Checked on 2026-10-17 with the build of the change, as for the bookmarks: the Pi in a headless labwc driven with
`wtype` and watched with `grim` (no pointer injection there), the Mac build with `--keys` and the new `--mouse` (mouse
events through `NSWindow.sendEvent` to the real handlers); the GTK build's new `--mouse` calls the page's drag and click
handlers the gestures call. The clipboard was read with `pbpaste` on the Mac.

- Keyboard, both: `v` boxes the first word of the page (in "Scroll" the first wholly shown), `j`/`l` move, `v` anchors,
  the bar "Highlight · Copy" (macOS "+ Share") shows over the selection or below it near the window's top; `j` past the
  page's last line turns the page and the selection goes on there; `Return` draws the marker in along its lines and
  keeps it; `Tab` reaches the highlight and `Return` offers "Delete · Copy", `d` deletes. The Mac: `c` puts
  "— Вы о ком? —" on the clipboard (the dash a word of its own).
- Mouse, both: a drag selects by words with both handles; a double click in the middle third selects a word ("отце.",
  "я."), a triple click its paragraph; a drag held at the right edge turns to the next page and the selection grows
  there.
- The list, both: the "Highlights" tab with the ochre rule, the text and "section · N%"; the Mac's share button. The
  Pi: `Return` goes to the highlight's page with the "← Back" pill. The Mac: `c` copies a highlight of three paragraphs
  as three lines. The Pi:
  a highlight across a hyphenated line ("чу-деса") stays on the same words after two larger text sizes and after a
  restart; it is in the state file.
- Cost on the Pi 5 (1920×1200, 123 Hz): `scripts/pi-headless.sh` 0 missed frames with the default, `ngl` and `gl`
  renderers, 2 with `vulkan`, idle 0 CPU ticks. Six paired runs with a highlight on every second paragraph and a bookmark
  on every one (`--marked`) and without: 0 and 2 late frames in 120 page turns, 1 and 0 in 120 scrolls. Strokes
  (`--strokes 20`, three runs per mode): 1–2 late frames per run, the first frame after a pause, as page turns have.
  200 highlights listed in at most 1.0 ms and saved in at most 0.4 ms (the corpus test on the Pi's books; on the Mac,
  with every corpus book's 200 bookmarks and 200 highlights in one state file, 24 000 lines, a write takes 15–17 ms:
  the file is re-read and written whole).
- Found and fixed: the release that ends a drag could reach the click handler after the drag's end (the order of GTK's
  two gestures), clearing the new selection; the release is now remembered as a drag's.
- Not checked by hand: a real pointer on the Pi (the gestures themselves), `Shift`+click and the drag out of the window
  in "Scroll" on both, the clipboard on the Pi (no `wl-paste` there), the Share menu and the menus Edit → Copy and
  View → Highlights on the Mac. The maintainer accepted the pass as it is (2026-10-17).

### The interface in Russian (`localization`, X5)

Checked on 2026-10-17 with the build of the change: the Pi with `LANG=ru_RU.UTF-8` and `en_US.UTF-8` in the headless
labwc (`wtype`, `grim`), the Mac build with `-AppleLanguages (ru)` and `(en)` and `--panel`/`--keys`.

- Both, in Russian: the window title "— Форзац"; the contents panel ("Содержание · Закладки · Цитаты", "Найти в
  оглавлении", "прочитано 12%"), the bookmarks and highlights lists and their empty hints; the settings sheet (the
  themes "Светлая … Лес", "Интервал", "Поля", "Листание" with "Сдвиг" and "Лента"); the search ("Поиск по книге",
  "455 совпадений"); the bars "Цитата · Копировать" (macOS "· Поделиться") and "Удалить · Копировать"; the note sheet
  "ПРИМЕЧАНИЕ 1"; the end page "Прочитано"; the help. The Pi also: the message "Не удалось открыть «photo.zip»: формат
  не поддерживается — нужен FB2, FB3 или EPUB" with "Закрыть", the recent books. In English the same screens read as
  before.
- Found and fixed: the Russian help was wider than the Mac window (long lines are wrapped now in both languages); the
  empty-list hint was cut to one line on the Mac (in English too); `←`/`→` did not switch tabs from the GTK bookmarks
  and highlights lists (the list view took the arrows); GTK's alert button came from GTK's own translations, missing on
  the Pi (the button is Forzac's string now).
- Cost on the Pi 5 (`scripts/pi-headless.sh`, 123 Hz): first page 151–154 ms in English and 149–157 ms in Russian
  (default renderer, three starts each); page turns 0 missed frames with the default, `ngl` and `gl` renderers, 1 with
  `vulkan`, in both languages; idle 0 CPU ticks in both.
- The Mac menus, printed with the new `--menus` from the `.app`: "Форзац" (О программе «Форзац», Настройки…, Скрыть,
  Завершить), "Файл", "Правка" (Копировать, Найти…), "Вид" (Содержание, Закладки, Цитаты, Страницы или лента, Крупнее,
  Мельче, Во весь экран), "Переход" (… Закладка), "Окно", "Справка"; AppKit's own items (Закрыть все, Эмодзи и символы)
  follow the language because `Info.plist` lists Russian. Found, not fixed (older than this change): "Enter Full Screen"
  appears twice in View, AppKit's and Forzac's.
- Not checked by hand: the Mac's alert for a file that cannot be opened (measurement runs show no alerts).

### The library (`library`, NP3 and P4 reversed)

Checked on 2026-10-17 with the build of the change. **The Pi 5 was not used**: it was busy with another change and its
SSH host key had changed; the maintainer chose to go on without it. So the 200 ms budget for 50 books on the Pi 5 is
not measured yet. The GTK shell was checked instead in the `ci/debian` container on the M1 (aarch64, software
rendering) under a headless labwc driven with `wtype` and `grim`, and with `--png` under a headless Weston; the Mac build
with `--library --png --keys --state` and `-AppleLanguages (ru)`. Test library: 50 entries over the EPUB/FB2 books of
the local corpus plus 8 small FB2s without covers (58 books).

- Both shells, in English and Russian: the head (brand, search circle, "Open a book" / "Открыть книгу"), the note line,
  the "Continue reading" / "Продолжить" card with the bar and share, the shelf with the books' own covers and generated
  ones (cloth and marbled, the paper label with title and authors), "Finished" / "Прочитано" under a finished book;
  the empty state with the sample page (Austen in English, Pushkin in Russian, hyphenated), side by side and, in a
  narrow window, one under the other (GTK); the Wave theme (Mac).
- Keys: `j` from the card to the shelf and down a row, `l`/`→` across, the selection scrolled into view; `/` and typing
  ("tolst" finds "Война и мир" by Лев Толстой), "No results" / "Ничего не найдено", `Esc` back to the whole library;
  `d` and `Delete` ask "Remove “Мы” from the library?" / "Убрать «Мы» из библиотеки?", `Esc` keeps the book, `Enter`
  removes it (the state loses the entry, the thumbnail goes); `Enter` opens the selected book and closes the library;
  `q` in the only book window brings the library back (GTK); `Ctrl+O` opens the file dialog with the "Books" filter
  (GTK); `Esc` with no book open closes the library and quits; `forzac k2.png` says "Couldn't open “k2.png”:
unsupported format — FB2, FB3 or EPUB needed" and the library follows (GTK).
- Speed in the container (release build, software rendering, not the Pi): the library of 58 books, covers cached,
  first frame at 178–217 ms from the start (warm); idle 0 CPU ticks in 10 s with the library open. Filling the cover
  cache for books read before the change ran in the background (one book at a time), the window showing first.
- Found and fixed on the way: in the GTK search field `l` moved the selection instead of being typed (the field's
  inner text widget, not the entry, has the focus); the sample page's last line ran into the folio; the Mac window's
  background was missing from `--png` and the brand showed a folder icon in a development build.
- Not checked by hand: the Pi 5 (all of it, including the 200 ms measurement), dropping a file on the window and the
  mouse (hover lift, "×", clicks) in both shells (no pointer injection here), the Mac's "after the last book" and `⌘O`
  from the library, a Russian keyboard layout in the library (by key code on the Mac, by GTK's Latin fallback, as the
  reader's keys).

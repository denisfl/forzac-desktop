## 0. Before

- [x] 0.1 `reading-log` and `reading-stats` archived (their Pi 5 tasks done or waived by the maintainer), so that the requirements this change rewrites exist in `openspec/specs`; `openspec validate library-sections --strict` passes after the archive

## 1. Core

- [x] 1.1 `i18n::library_help(platform)`: sections "Books", "Reading stats", "Window" with each shell's keys (`⌘1`/`⌘2`, `Ctrl+1`/`Ctrl+2`, `Ctrl+,`, `Ctrl+?`, `F10` on Linux only) and the support line; both languages; tests like `help`'s (every key the library handles is listed, nothing else; Russian strings present)
- [x] 1.2 (added with each shell's code: the catalogue test rejects strings nobody asks for) Strings for the sections, the toolbar and header ("Books", "Reading stats", "Open a Book…", "Add a Folder…", "Save Image…", "Main Menu", "Keyboard Shortcuts", "About Forzac", "General", "Books" pane, "Theme", the six theme names, "Show"); catalogue test passes
- [x] 1.3 C ABI `forzac_library_help(platform)` as `forzac_help`; `macos/forzac.h`; `cargo test -p forzac-core`, clippy and fmt pass

## 2. macOS: the window and its toolbar

- [x] 2.1 (2026-10-23: the section and `--keys s` screenshots; the end page line goes through `openStats`, not clicked by hand) Stats as a view: `macos/Stats.swift` from an `NSWindow` to an `NSView` in the library's content, period and scroll kept while hidden; the end page's month line shows the library window on "Reading stats" (opening it if closed) — check: the month line with the library closed and open
- [x] 2.2 (2026-10-23: screenshots at 1040 × 760 and 520 × 420, the search field a button when narrow) The toolbar: transparent unified title bar over the paper, window title = section name (hidden in the toolbar), `NSSegmentedControl` "Books | Reading stats" centred, trailing "+" (`NSMenuToolbarItem`: Open a Book…, Add a Folder…) and `NSSearchToolbarItem` in Books, `NSSharingServicePickerToolbarItem` in Reading stats; the brand, "⋯" and the old search field removed; `minSize` 520 × 420 — check: screenshots at 1040 × 760 and 520 × 420 in Light and Dark, the search field collapsing to a button
- [x] 2.3 (2026-10-23: `⌘2`, `Esc`, `/` + typing via `--keys` and by hand; `⌘,` from the menu; `⌘,`, `⌘F` and the Russian layout checked by hand by the maintainer on 2026-10-23) Keys and menus: `s`, `Esc`, `⌘1`/`⌘2`, `⌘F` and `/`; File › Add a Folder… `⇧⌘O`, Show the Books Folder; View › Books / Reading stats with the check mark; Edit › Find — check: each menu item and key by hand in English and Russian layouts
- [x] 2.4 (2026-10-23: `--library --state` of a scratch state with `update-latest` 1.0.0: screenshots at 1040 and 520 wide, Light and Dark, English and Russian; a version of 30 numbers showed the line clipped without "…" and its link cut off — fixed, the line's paragraph style truncates the tail; checked again at 520) The update line under the toolbar, above the tabs, elided when narrow — check: with a local appcast newer than the build (as `docs/publishing.md` describes), at 1040 and 520 wide
- [x] 2.5 (2026-10-23: `--keys ?` screenshot) `?` shows the keys card with `forzac_library_help` over the library — check: screenshot, `?` and `Esc` close it

## 3. macOS: settings and theme

- [x] 3.1 (2026-10-23: both panes on screen, the pane titles the window) Settings window: toolbar panes "General" (Theme, Language) and "Books" (Sync with the folder's "Show" and "Change…"), last pane restored, title by pane, not resizable, minimise and zoom dimmed — check: screenshots of both panes, reopen restores the pane
- [x] 3.2 (2026-10-23: Wave chosen in Settings repainted the library at once; with a book open — in 7.1) Theme swatches (paper, "Aa" in ink, name) writing the reading theme; a notification repaints the library and every open book window — check: change the theme with a book open on a numbered page; the page and number stay
- [x] 3.3 (2026-10-23: 60 fps recordings (`screencapture -v`) of the library and of a book opening in the Dark theme; by each frame's brightness, the first frame that shows either window is already the dark paper) First frame in paper: library and book windows set background and appearance before ordering front — check: a screen recording of opening each in Dark shows no white frame

## 4. Linux: the header and the sections

- [x] 4.1 (2026-10-23: the section under Weston, `FORZAC_SECTION=1`; the end page's line goes through `show_stats`, not clicked) Stats as a widget in a `gtk::Stack` with the books; the end page's month line presents the library on "Reading stats" — check as 2.1
- [x] 4.2 (2026-10-23: under a headless Weston with the Hyprland header row; the floating title bar on GNOME not shot) The header widget: start "+" (menu Open a Book… `Ctrl+O`, Add a Folder… `Ctrl+Shift+O`) and search toggle in Books, "Save Image…" in Reading stats; centre switcher; end main menu (`MenuButton` primary, `F10`) with Settings `Ctrl+,`, Keyboard Shortcuts `Ctrl+?`, About Forzac; set as the title bar on floating compositors, the top row on Hyprland; brand and "⋯" removed — check: screenshots on GNOME (in the Arch container's Weston) and under Hyprland on the Omarchy laptop
- [x] 4.3 (2026-10-23: screenshots at 1040 and 470 wide, books and stats; 551/549/360 and a live resize left to 7.2) Narrow windows: below 550 logical px the switcher moves to a bottom pill and the header shows the section's name; content bottom padding while the pill shows; works down to 360 × 294 — check: screenshots at 1040, 551, 549, 470 and 360 wide, selection and scroll kept across 550
- [ ] 4.4 (2026-10-23, the English layout under Xvfb with xdotool: `/` + "pratch" and `Ctrl+F` + "chip" filter live, `Esc` closes the bar; the Russian layout left to the maintainer — Xvfb takes no second layout group, and xdotool types Cyrillic through a spare keycode no physical key has) Search bar under the header (`Ctrl+F`, `/`, the toggle; no type-to-search), live results, `Esc` closes — check: `/pushk` and `Ctrl+F` in both layouts
- [ ] 4.5 (2026-10-23, the English layout under Xvfb: `s`, `Esc` back, `Ctrl+2`/`Ctrl+1`, `Ctrl+,`, `?` and `Ctrl+?`, `F10`; `Esc` with the keys card open closed the window, and every key reached the library under the card — fixed, the card's key controller is added after the shortcuts, which run first otherwise; the Russian layout as in 4.4) Keys: `s`, `Esc`, `Ctrl+1`/`Ctrl+2`, `Ctrl+,`, `Ctrl+?`/`?` (the keys card with `forzac_library_help`) — check by hand in both layouts
- [x] 4.6 (2026-10-23: under Xvfb with `update-latest` in a scratch state: 1040 and 470 wide, Light and Dark; a long version elided with "…"; the line's link took the window's first focus and wore a focus ring after any key — now not focusable, as the Mac's) The update line under the header, elided when narrow — check: with a fake newer release at 1040 and 470 wide

## 5. Linux: settings, theme, about

- [x] 5.1 (2026-10-23: screenshots wide and at 470; the sheet scrolls when taller than the window) Settings sheet: sections Theme, Language, Sync (folder "Show" and "Change…"), modal; full-width from the bottom below 550 px — check: screenshots wide and at 470
- [x] 5.2 (2026-10-23: under Xvfb, one process over D-Bus: Wave chosen in the library's settings repainted the library and an open book at once, the book staying on page 5 of 373) Theme swatches writing the reading theme; the application repaints every window — check as 3.2
- [x] 5.3 (2026-10-23: a sheet, as the settings, which closes the design's open question) About Forzac: version, MPL-2.0, source link, the support line, in the theme (resolve design's open question by screenshots in the six themes)
- [x] 5.4 (2026-10-23: a 60 fps x11grab recording under Xvfb on a black root, Dark theme: the first frame of the library and of a book window is the dark paper, no white frame; Wayland compositors not recorded) First frame in paper for library and book windows — check: no white frame when opening in Dark (screen recording)

## 6. Docs

- [x] 6.1 `README.md` keys and the library's description; `docs/roadmap.md`; `docs/android-parity.md` (the sections and Theme in Settings now match the phone) — check: the README keys table lists every key of `library_help`

## 7. Check on hardware

- [ ] 7.1 (2026-10-23, by the maintainer: `⌘,`, `⌘F`, the Russian layout, the theme with a book open, the settings' margins fixed; Dark and English screens and idle CPU still to do; on 2026-10-23 the library at 1040 and 520, the update line, the stats and the keys card shot in Dark and English with `--png`, and idle measured with `proc_pid_rusage` instead of Activity Monitor: 8.7 ms of CPU and 8 wakeups in 30 s, no Forzac code on the main thread by `sample` — the Settings window in Dark and English is left) Mac (M1, macOS 14): the screens of 2.2–3.3 in English and Russian, Light and Dark; nothing drawn while idle (Activity Monitor CPU 0 % in the library for 30 s)
- [ ] 7.2 Omarchy laptop (Hyprland, scale 2): the library tiled at two, three and four windows; screenshots; `F10`, `Ctrl+,`, `Ctrl+2`, `Esc` by hand
- [ ] 7.3 Pi 5 (with the maintainer's consent): `scripts/pi-headless.sh` — switching to "Reading stats" with a year of log within 100 ms, a live resize across 550 px without missed frames, library first frame still ≤ 200 ms with 50 books (`forzac --measure`)

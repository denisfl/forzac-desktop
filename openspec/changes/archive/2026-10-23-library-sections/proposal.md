## Why

The Android app's 0.9.11 made Books and Reading stats two equal sections with one action each, cut its header to a
title and two buttons, and let the reader change the theme without a book open; the maintainer likes it and asked
(2026-10-23) for the same on the desktop, "designed with the guidelines, the system's ways and the window size in
mind". Today the desktop library keeps the stats in a separate window and in a "⋯" menu with the Books folder items,
the theme can be changed only from a book, and the Mac settings window is the only window outside the reader's look.

The guidelines checked on 2026-10-23 (`design.md`, "Sources") rule out copying the phone: Apple asks not to put
controls at the bottom of a Mac window and to switch views with a segmented control in the toolbar; GNOME puts a view
switcher in the header bar and moves it to a bottom bar only when the window is narrow — which is the phone's pill,
and on Omarchy's tiled laptop screens a window under 550 px is common.

## What Changes

- **Two sections in the library window**: "Books" ("Книги") — the library as it is — and "Reading stats"
  ("Статистика") — the stats window's content, which stops being a window of its own. The end page's month line opens
  the library window on "Reading stats".
- **macOS**: the library window gets a unified toolbar over the reader's paper: a segmented control "Books | Reading
  stats" in the centre; at the trailing edge "+" (Open a Book…, Add a Folder…) and a search field in Books, Share in
  Reading stats. No settings button in the toolbar and no bar at the bottom at any width. Every command is in the menu
  bar: View › Books `⌘1`, View › Reading stats `⌘2`, Edit › Find `⌘F`, File › Open… `⌘O`, Add a Folder… `⇧⌘O`, Show
  the Books Folder, Change the Books Folder…. The window's title is the section's name, not "Forzac".
- **macOS settings**: the Settings window (`⌘,`) keeps the system's look and controls, and gets a toolbar of two panes —
  "General" (Theme, Language) and "Books" (the Books folder and Sync); it remembers the last pane and is titled by it.
- **Linux**: the header — the window's header bar on floating compositors, a header row on Hyprland — holds "+" and the
  section's action at the start (search in Books, Save Image in Reading stats), a switcher "Books | Reading stats" in the
  centre and the main menu "☰" at the end: Settings `Ctrl+,`, Keyboard Shortcuts `Ctrl+?`, About Forzac. Search becomes
  a bar that slides down under the header (`Ctrl+F`, `/`). Below 550 logical pixels of window width the switcher leaves
  the header for a floating pill at the bottom, and the header shows the section's name.
- **The "⋯" menu goes away** on both: its Books folder items move to Settings (Linux) and the File menu and Settings
  (macOS), "Add a Folder…" to "+", "Reading stats" to the switcher. The brand ("Forzac" with the icon) leaves the head.
- **Theme everywhere**: a Theme choice in Settings — the six themes of "Aa", the same saved theme, applied at once to
  the library and open books; every window shows the theme's paper from its first frame. Window chrome follows each
  system: native on macOS (toolbar, menus, settings), in the theme's colours on Linux.
- **Keys**: `s` in Books opens Reading stats, `Esc` there returns to Books (`s` there still shares or saves the
  picture); `⌘1`/`⌘2` on macOS and `Ctrl+1`/`Ctrl+2` on Linux choose the section; `Ctrl+F`/`⌘F` and `/` search.
- **Window size**: the library supports windows down to 360 × 294 logical pixels on Linux and 520 × 420 points on
  macOS (its minimum), laid out for narrow, medium and wide widths; the update line moves under the header.
- No new setting: the theme in Settings is the existing reading theme, reachable without a book.

## Non-goals

- A sidebar, more sections, or moving the views (Recent, Title, Author, Series, Shelves) into the switcher.
- libadwaita: the GTK shell keeps its own widgets and imitates the patterns (`design.md`, decision 1).
- The book window and the macOS reading bar, the web reader, the Android app.
- Restyling the macOS system chrome (toolbar materials, menus) in the reader's colours.

## Capabilities

### New Capabilities

### Modified Capabilities
- `library`: the window holds two sections with a per-platform header (toolbar, header bar, bottom pill when narrow);
  the "⋯" menu is removed; keys, search and the empty library follow.
- `reading-stats`: "The stats window" becomes the "Reading stats" section of the library window (the requirement is added
  by the change `reading-stats`, which must be archived first).
- `reader-navigation`: the end page's month line opens the library window on "Reading stats" (added by `reading-stats`).
- `settings`: Theme in Settings; macOS panes "General" and "Books"; the Books folder's items in Settings on Linux;
  `Ctrl+,` on Linux.
- `books-folder`: "Show the Books folder" and "Change the Books folder…" are reached from Settings and the File menu,
  not "⋯".
- `macos-app`: the menu bar's View, Edit and File items for the library.
- `linux-integration`: the header on floating compositors and on Hyprland; the smallest supported window.
- `update-check`: the line moves from the head to under the header.
- `help`: the keys card in the library window (`?`, and the main menu on Linux).

## Impact

- Shells only: `macos/Library.swift` (toolbar, sections, search field), `macos/Stats.swift` (from a window to a view),
  `macos/Settings.swift` (panes, theme), `macos/main.swift` (menus); `gtk/src/library.rs` (header, switcher, bottom
  pill, search bar, main menu), `gtk/src/stats.rs` (a section), the settings sheet, an About dialog.
- Core: the strings of both languages and the library's keys card (`core/src/i18n.rs`); nothing in layout, sync or the
  state format. The C ABI gains one call, `forzac_library_help` (the theme setting and the stats already cross it).
- Must follow the archive of `reading-log` and `reading-stats` (their last task: the Pi 5 measurement).
- Docs: `README.md` (keys), `docs/roadmap.md`.

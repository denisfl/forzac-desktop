## Context

Today (0.17.0) the library window has its own head drawn in the content on both shells: the brand, the update line, "⋯",
search and "Open a book". On macOS it sits under the standard title bar "Forzac" (`macos/Library.swift`, 1040 × 760, no
minimum size); on Linux the window is undecorated on Hyprland and keeps the toolkit's title bar elsewhere
(`gtk/src/library.rs`). "Reading stats" is a window of its own (`macos/Stats.swift` 520 × 760, minimum 420 × 560;
`gtk/src/stats.rs` 520 wide). Settings are a system-styled window on macOS (`macos/Settings.swift`) and a themed sheet on
Linux. The theme is changed only from a book ("Aa", `t`) and stored as one setting shared by all windows
(`forzac_settings_theme`, `forzac_settings_step`). The keys card (`help`) exists only in book windows; its text comes
from the core (`i18n::help`, `forzac_help`).

This change depends on `reading-log` and `reading-stats` being archived: it rewrites requirements they add (the stats
window, the end page's month line, the library's keys and menu).

## Sources

Checked on 2026-10-23 (Apple's pages through their JSON data, the rest directly):

- Apple HIG — Tab bars ("No additional considerations for macOS"), Segmented controls ("Consider using a segmented
  control to help people switch views in a toolbar"; nouns as labels; text or images, not both), Toolbars (centre for
  common controls; trailing for search and More; "Don't title windows with your app name"; Liquid Glass: fewer custom
  backgrounds and tints), Layout and Windows ("Avoid placing controls or critical information at the bottom of a
  window"), Settings (`⌘,`, a non-customisable toolbar of panes, restore the last pane, title by pane, dim minimise and
  zoom, "Avoid adding settings buttons to a window's toolbar"), The menu bar (every command in the menu bar even when
  it is elsewhere), Search fields (trailing side of the toolbar). https://developer.apple.com/design/human-interface-guidelines/
- AppKit — `NSToolbar.centeredItemIdentifiers` (macOS 13), `NSSearchToolbarItem` (collapses to a button when space is
  short, macOS 11), `NSSharingServicePickerToolbarItem`, `NSMenuToolbarItem`.
- GNOME HIG — Header bars (main actions at the start, the view switcher in the centre, menus at the end, few controls),
  Menus (primary menu "Main Menu": Preferences, Keyboard Shortcuts, Help, About; no Quit or Close), View switchers (3–5
  views; "switch to the bottom window edge if the window becomes too narrow"), Adaptive (support 1024 × 600; phone-capable
  apps down to 360 × 294; windows get tiled), Search (a bar that slides down under the header; `Ctrl+F`, a toggle
  button), Keyboard (`Ctrl+,`, `Ctrl+?`, `Ctrl+F`, `F10`; no `Alt` shortcuts), UI styling (keep custom styling to a
  minimum). https://developer.gnome.org/hig/
- libadwaita 1.10 — `AdwViewSwitcher` / `AdwViewSwitcherBar` with a breakpoint `max-width: 550sp` in the docs' example;
  `AdwDialog` floats or becomes a bottom sheet by size. https://gnome.pages.gitlab.gnome.org/libadwaita/doc/1-latest/
- Omarchy (`omacom/omarchy`, branch `quattro`): Hyprland dwindle layout, gaps 5/10, `GDK_SCALE = 2` on HiDPI. Estimated
  tiles: a 1440 × 900-logical laptop gives two windows about 710 px wide and three or four under 600 px; 1920 × 1080 at
  scale 1 gives halves of about 940 px and quarters of about 465 px.

## Goals / Non-Goals

**Goals:** each shell's head follows its own platform; the same two sections, the same words and the same per-section
action on both and on Android; the library usable from 360 px wide on Linux; no new setting; no drawing while idle.

**Non-Goals:** sharing UI code between the shells; a responsive layout for book windows; animating the move between
the header and the bottom pill.

## Decisions

### 1. GTK keeps its own widgets; the GNOME patterns are imitated

The GTK shell uses plain GTK 4.14 widgets themed by its own CSS (`set_theme`), not libadwaita. Adding libadwaita for
`AdwViewSwitcher`, `AdwBreakpoint` and `AdwDialog` would bring its style sheet, which fights the reading themes, and a
dependency the Pi 5 and the AUR package do not need yet. The same behaviour is cheap to build: a two-button linked box
for the switcher, a `gtk::Overlay` for the bottom pill, a `gtk::SearchBar` under the header, a `gtk::MenuButton` with
`set_primary(true)` (so `F10` opens it) and a `gio::Menu` for the main menu. *Alternative:* libadwaita 1.5+ —
reconsider if the shell ever adopts its look wholesale.

### 2. Where the switcher lives on each platform

- **macOS**: an `NSSegmentedControl` (two text segments, `.selectOne`) in a unified `NSToolbar`, listed in
  `centeredItemIdentifiers`; the window `titleVisibility = .hidden`, `titlebarAppearsTransparent`,
  `.fullSizeContentView`, as the reading bar already does for book windows. No bottom bar at any width — the HIG asks
  for none, and Mac windows are rarely narrow. *Alternative:* `NSTabViewController` with `.toolbar` style — it makes
  icon-and-label toolbar tabs, the Settings-window look, wrong for a main window; a tab view inside the content —
  rejected because its tabs would sit next to the views' tabs (Recent, Author…) and read as one more of them.
- **Linux**: the switcher in the header's centre while the window is at least 550 logical pixels wide; below, in a pill
  at the bottom over the content, and the header's centre shows the section's name. 550 is libadwaita's example
  breakpoint and leaves room for "+", search, the switcher and the menu at the header's 48 px height. The decision
  reads the window's width in `size_allocate` of the library's root and only toggles visibility (no re-parenting of the
  grid), so a live resize costs one relayout; a 16 px hysteresis is not needed because the switcher's size does not
  change the window's width.
- **Two segments, though GNOME's view switcher is for 3–5 views**: the pair is what the reader knows from the phone,
  and GNOME's own two-option control (a toggle group) looks the same; the views (Recent, Author…) stay tabs inside
  "Books" because they are orderings of one list, not places.

### 3. The header on Linux: title bar or top row

On floating compositors the header widget is set with `window.set_titlebar()` as a `gtk::HeaderBar` whose title widget
is the switcher and whose start and end hold our buttons (its window buttons shown); on Hyprland the same widget is
put as the first child of the window's box with `show_title_buttons(false)` and the window stays undecorated. One
widget, two placements, chosen once at start (`HYPRLAND_INSTANCE_SIGNATURE`, as today).

### 4. Stats become a view, not a window

`macos/Stats.swift` turns its `NSWindow` into an `NSView` placed in the library's content beside the books' view;
`gtk/src/stats.rs` turns its `gtk::Window` into a widget in a `gtk::Stack` with the books. Each keeps its state (period,
scroll) while hidden; the core's stats call (`forzac_stats_*`, from `reading-stats`) is unchanged. The end page's month
line asks the app to show the library window on "Reading stats" (macOS: the app delegate's existing "show library";
Linux: the existing `OPEN` handle) instead of opening a stats window.

### 5. Search per platform

macOS uses an `NSSearchToolbarItem` (trailing, collapses to a button in a narrow window; `⌘F` and `/` focus it); the
old serif field that replaced the brand goes away with the brand. Linux uses a `gtk::SearchBar` under the header with
`key_capture_widget` unset: letters are commands in the library (`j`, `d`, `s`…), so typing does not start a search —
a deliberate departure from GNOME's type-to-search. The search logic stays in the core (`library::search`).

### 6. Theme in Settings and everywhere

- The theme choice writes the same setting as "Aa" (`theme` in the state, through `forzac_settings_step` on macOS and
  the shared `Settings` on Linux). Each shell already repaints a window when its theme changes; Settings now broadcasts
  the change to all windows (macOS: a `NotificationCenter` post observed by the library and every book view; GTK: the
  application iterates its windows and calls their `set_theme`). A redraw only, as the invariant says.
- First frame: macOS sets `backgroundColor` and `appearance` before `makeKeyAndOrderFront` (the library does; book and
  stats windows are checked); GTK sets the window's theme CSS before `present()` — the library does, book windows are
  checked and fixed if they show a white frame.
- Chrome: macOS keeps the system's toolbar, menus and settings controls, matched only by the window's appearance
  (`.darkAqua` for dark paper) — Apple asks to avoid custom toolbar backgrounds and tints, and the system's Liquid Glass
  in macOS 26 needs its own materials. Linux draws header, menus, popovers, sheets and the pill in the theme's colours,
  departing from GNOME's "keep custom styling to a minimum" on purpose: the library is a reading surface, and the
  phone and the web reader look the same way.
- The swatches on macOS are custom-drawn buttons (paper with "Aa" in ink, as `Panels.swift` draws the theme circles) in
  an otherwise system-styled settings pane.

### 7. Keys card in the library

The core gains `i18n::library_help(platform)` with the sections "Books", "Reading stats" and "Window", tested like
`help`, and a C ABI call `forzac_library_help(platform)` returning the same text format as `forzac_help`; the shells
reuse their help card with that text. This is the only new call across the C ABI.

### 8. Menu bar on macOS

The File menu gains Add a Folder… `⇧⌘O` and Show the Books Folder (Change the Books Folder… is there already); the View
menu gets Books `⌘1` and Reading stats `⌘2` with a check mark when the library is the key window (HIG does not list
`⌘1`…, but Finder and Mail use them for views); Edit › Find `⌘F` focuses the library's search field. The "+" toolbar
menu repeats Open… and Add a Folder….

## Risks / Trade-offs

- [The bottom pill covers the last row of covers or the stats' list] → the scrolled content gets bottom padding equal
  to the pill's height plus 16 px while the pill shows; checked at 360 × 294 and 470 × 900.
- [`NSToolbar` centred items and a transparent title bar behave differently on macOS 14 and 26] → screenshots on the
  M1 with macOS 14; macOS 26 checked when the Mac is updated, the items are all standard ones.
- [A header bar as the title bar changes how the window is dragged and double-clicked on GNOME] → `gtk::HeaderBar`
  keeps the toolkit's behaviour; on Hyprland there is no title bar to drag.
- [Readers used to the stats window lose a window they could place beside a book] → the end page's line still shows
  the stats in one click; if missed, a later change can add "Open in New Window" (not now: one window is simpler).
- [Removing the brand from the head loses identity] → the window and Dock/taskbar icon, the empty library and About
  keep it.

## Migration Plan

No state changes: the section is not remembered (the window opens on "Books"), the theme setting is the existing one.
`stats.window` sizes saved by 0.17 (if any) are ignored. Rollback is a previous release.

## Open Questions

- Whether "About Forzac" on Linux is the toolkit's `gtk::AboutDialog` themed by CSS or a sheet like Settings — either
  satisfies the spec; decide by how it looks in the six themes.

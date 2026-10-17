# Roadmap

From the spike ([docs/spike.md](spike.md)) to 1.0. Each row is an OpenSpec change in `openspec/changes/<name>`
(`proposal → specs → design → tasks`, then implementation and `openspec archive`). Sizes are rough, for one developer,
and get refined in each change's design.

Linux comes first: Omarchy and the Raspberry Pi 5 are the primary audience. macOS follows once the reader is complete
on Linux, so the shared core API is settled before the second shell catches up.

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
| 11a | `macos-release` | An ad-hoc signed `.app` in a DMG to download from forzac.truenrth.app, with first-open instructions (no Developer ID, App Store or Homebrew) | On a fresh Mac user account the DMG from the site opens by following the instructions | 1–2 d |

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
| 14 | `release-1-0` | Performance budgets checked on the Pi 5 and the Mac; x86_64 packages checked in containers; release notes; packages published | Budgets met on the Pi 5 and the Mac | 3–4 d |

Total: about 18–22 weeks.

## After 1.0

- `accessibility`: the page's text for screen readers (Orca, VoiceOver), as TalkBack reads it on Android.

## Not planned for 1.0

Confirmed with the maintainer in `android-parity` (2026-10-16). Text selection and quotes left the list on 2026-10-17,
when the maintainer asked for them (`selection-quotes`).

PDF; a library screen with import and covers
(books open in place; recent books are enough); dictionaries; formulas (MathML); fixed-layout EPUB; Flatpak (AUR and
the `.deb` first); a Developer ID, notarisation, Homebrew and the App Store; following the Omarchy theme colours (a possible later change).

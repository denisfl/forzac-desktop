## Context

[`docs/android-parity.md`](../../../docs/android-parity.md) lists every reader feature of the Android app next to the
desktop app and records the maintainer's decision for each difference (2026-10-16). This change closes the small gaps
decided there; the larger ones are their own changes (`viewer`, `two-page-spread`, `accessibility`). The Android code
named below is the reference for behaviour and wording; the row numbers (T3, N6, …) are the list's.

## Goals / Non-Goals

**Goals:** the gaps assigned to `android-parity` in the list, in both shells; a by-hand pass on the Raspberry Pi 5 and
the M1 MacBook Pro against the whole list.

**Non-Goals:** anything the list keeps as it is or splits off; no new settings; no change to the reading-state file
format.

## Decisions

**Language from the text (T3) — core.** `parse` fills `Book::language` when the book names none, with a port of
Android's `guessLanguage` (`BookLanguage.kt`): the first 2000 letters of paragraphs and headings, Cyrillic over half →
`ru`, else `en`. Done once after parsing, so the hyphenator, the corpus test and both shells get it without change.
`Hyphenator::for_language` keeps `ru` and `en` only; another language returns `None` (no hyphenation), and the `ru`
default for a missing language goes, since there is none any more. The `localization` proposal drops this item.

**Monospace and strikethrough (T6, T7) — core, fonts in both shells.** PT Mono Regular (ParaType, OFL 1.1, Cyrillic)
goes into `assets/fonts/pt-mono/` with its licence. `fonts.rs` registers it like the optional families (a `FontId`
on first use, `font_source` for the shells); it has one face, so code inside emphasis or strong stays regular rather
than synthesised. `Kind::Code` paragraphs and `Style::Code` ranges shape with it (0.85 and 0.9 of the size). The
glyph fallback still applies inside code. Strikethrough becomes strokes: `PageContent` gets `strokes: Vec<Stroke>`
(`rect`, `ink`), one per line segment, at half the x-height above the baseline and `size × 0.06` thick, emitted where
lines are drawn so pages, the column and notes all have them. Shells draw strokes as filled rectangles in the stroke's
ink, like rules. C ABI: `ForzacPage` gains `strokes`/`stroke_count`. GTK's fontconfig configuration and
`scripts/package-linux.sh` list the new directory; the Mac takes the bytes from the core as for every font.

**Clicks in the side thirds (M2) — shells.** In "Slide", a released click that hits no area (marker, link) and comes
while no panel is open turns back in the left third and forward in the right third, through the same `turn` the keys
use; the middle third does nothing. Drags are not clicks (GTK `GestureClick` already ignores them; AppKit checks the
mouse moved less than a few points). "Scroll" does not turn by click.

**The end page (M7) — core content, shell placement.** The core renders the end page for a style:
`end_page(book, style) -> PageContent` — title (bold, 1.35), authors, "Finished" in the secondary ink, centred in the
window, in the book face. C ABI `forzac_end_page`. In "Slide" the shell treats it as one page after the last page of
the last chapter: the turn slides to it, back slides to the text, forward does nothing. In "Scroll" it is one more
tile, a window high, after the last chapter's column. Showing it saves the position as finished:
`Store::save_position(…, finished: bool)` stores the last page's start with a progress of 1.0 instead of the
computed share; the state file already keeps the progress per book, so its format does not change. The contents read "100% read" when the book is
finished and the reader is on its last page. Reopening a finished book opens its last page of text, as Android.

**Untitled chapters (N2) — core.** `Book::contents` with no `toc` lists every chapter, an untitled one as
"Chapter N". The string is English in the core; `localization` moves the wording into the shells.

**Short link targets (N6) — core rule, shells' note sheet.** A port of Android's `Book.preview`
(`LinkPreview.kt`): a `LinkTarget::Place` with an `end` in the same chapter, at most 3 blocks and 600 characters of
text, yields those blocks. `layout_note` generalises to `layout_blocks(book, blocks, …)`, which notes and previews
share. The view's `open` sends `Open::Preview(key)` (GTK) / `.preview(key)` (Mac) instead of jumping when a preview
exists; the note sheet shows it without a label. C ABI: `forzac_link_preview(doc, key, width, settings, out) -> f32`
with the shape of `forzac_note`. A preview is not a jump: nothing goes on the `Ctrl+O` stack.

**The "← Back" pill (N8) — shells, over the existing stack.** The view remembers, with the top of its back stack,
where the last jump arrived. The pill is drawn by the shell over the foot (centred where the page number is, 18 px
radius, 14 px Golos Text, the ink as its fill and the paper as its text, Android's `ReturnPill`) while the stack is
not empty and the current place is fewer than two pages (in "Scroll" two window heights) from the arrival. A click on
it calls `back`. It fades in and out in 150 ms and is not drawn otherwise, so it costs nothing at rest. Search matches
reached with `n`/`N` do not count as new jumps (as today).

**Search in table cells (S2) — core.** A table row's text is its cells joined by `\n`, as Android's `TableRow.text`;
a match is a `Locator` into that text, so it never crosses a cell (the query cannot contain `\n`). `marks` already maps
offsets to glyphs per line; table cells keep their glyph offsets shifted by the cell's start in the row text, so
highlights land in the cell. `Block::text_len` counts the separators, so progress stays consistent with the offsets.

**Theme cross-fade (C4) — shells.** GTK: on a theme change the view keeps the old page node, and for 200 ms on the
frame clock draws `gsk::CrossFadeNode(old, new, t)`; panels use a CSS `transition` on their colours. Each frame only
composes two cached nodes. macOS: a `CATransition` of type fade, 0.2 s, on the window's content layer around the
recolouring. Nothing runs after the fade.

**Recent books in the theme (C10, D2) — shells.** GTK gives the recent window the panels' CSS classes
(`forzac-panel`, the current palette from the settings and the system appearance), titles in PT Serif and details in
Golos Text. macOS colours its table the same way and adds `j`/`k`, `q` and `Esc` to its key monitor.

**Errors with reasons (E1) — core reasons, shells' messages.** `Error` gains `Damaged` (a zip that cannot be read
as an archive) and `Unavailable` (the file cannot be read); `open` maps I/O errors to `Unavailable`. C ABI:
`forzac_open(path, error: *mut u32)` writes the reason when it returns null. Messages,
in the Android wording without PDF: GTK a `gtk::AlertDialog` (the application held while it shows, so a failed
`xdg-open` still shows it), macOS the existing `NSAlert` with the reason as its informative text. After the message,
with no book window open, the recent books show.

**One window per book and after the last one (D1, P5) — GTK.** The application keeps its book windows by canonical
path and presents the existing one. When a book window closes and it was the last window, the recent books open;
closing the recent books quits, as it does now. macOS already behaves this way.

**Keeping the screen on (X2) — shells.** GTK: `Application::inhibit(window, IDLE, "Reading")` while a book window is
active and was touched in the last 10 minutes (key, turn, scroll, click), `uninhibit` 10 minutes after the last
touch or when the window loses focus. On Wayland GTK uses the compositor's idle-inhibit protocol, which labwc
supports. macOS: `ProcessInfo.beginActivity(options: .idleDisplaySleepDisabled, reason:)`, ended the same way. One
timer per window, reset on input; nothing is polled.

## Risks / Trade-offs

- **Idle inhibition on labwc and Hyprland** depends on GTK reaching the idle-inhibit protocol; checked by hand on
  the Pi (task 6). If it does not work there, the fallback is `org.freedesktop.portal.Inhibit`, and the row stays a
  known gap on that compositor.
- **"Chapter N" in English** in the core until `localization`.
- **PT Mono has no italic or bold**: emphasised code is regular. Android uses the system monospace, which has them.
- **The pill and the stack** are two ideas at once (Android has one return point, the desktop a stack); the pill only
  advertises the top of the stack.

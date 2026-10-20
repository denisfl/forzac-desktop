## Context

A macOS book window (`Reader` in `macos/main.swift`) is a titled `NSWindow` whose content view (`container`) holds the
`PageView`, the help, the panels and, when open, the reading menu's two bars (`ReaderMenu` in `macos/Menu.swift`).
The page's style comes from the core: a 56 px top margin with the running head centred in it, the text below
(`PageStyle::column`). The title bar is the system's, 28 points, always shown.

Two prototypes on the M1 (macOS 14) settled how the bar can be built:

- Window buttons can be moved down to the middle of a 52-point bar by hand, but AppKit puts them back at the first
  relayout of the title bar (a title or appearance change): too fragile.
- An empty `NSToolbar` in the unified style makes the title bar 52 points with the window buttons centred, the system
  way. With `fullSizeContentView`, a transparent title bar and no separator, the toolbar draws only its items over the
  page. But the toolbar takes every click in the top 52 points, even at alpha 0; hiding the title bar's container view
  (`isHidden`) gives those clicks back to the page, and stays hidden across relayouts.

Maintainer's decisions (2026-10-20): one top bar (the reading menu shows this one); the bar fades 2 s after the pointer
stops. After a first version on screenshots — the section over the book's title in the middle, over a strip of paper
that hid the running head — the maintainer found that the bar repeated the page (the titles the head, the filled
bookmark the ribbon) and chose a transparent bar around the page's own head.

## Goals / Non-Goals

**Goals:** the page alone on screen while reading; the window's controls one pointer move away; system-native look and
behaviour (dragging, double click, tooltips, VoiceOver labels); no change to the page's layout or to the core; nothing
running while idle.

**Non-Goals:** Linux; panels under the bar; a customisable toolbar; showing the bar from the keyboard other than through
the reading menu.

## Decisions

**1. The window.** The book window gets `.fullSizeContentView`, `titlebarAppearsTransparent`, `titleVisibility =
.hidden`, `titlebarSeparatorStyle = .none`, and an `NSToolbar` in the `.unified` style. The `PageView` fills the whole
content view, as now; the page grows by 28 points, which is an ordinary window resize for the layout (the position is
kept). Alternatives: moving the window buttons by hand (reset by AppKit, see Context); our own bar without a toolbar
(the buttons cannot be centred in it robustly).

**2. The bar is the toolbar.** Items: "Back to library" (`chevron.left`), a flexible space, Contents (`list.bullet`),
Text settings (`textformat.size`), Search (`magnifyingglass`), Bookmark (`bookmark`, `bookmark.fill`). SF Symbols
rather than Lucide here: these are the system's toolbar buttons next to the system's window buttons, as in Apple Books;
the reading menu's own controls and the panels keep their Lucide icons. No titles: the page's running head is the
title. Display mode icon only; the toolbar cannot be customised. The labels and tooltips go through `tr` (all already
in the catalogue).

**3. Paper under the buttons only.** A layer-backed view across the bar's 52 points holds a horizontal gradient of the
theme's paper: opaque under the left group (the window buttons, measured from the zoom button, plus the back button) and
the right group (four buttons), clear over 24 points towards the middle. A head shorter than the space between them is
untouched; a longer one fades out under the buttons. The ribbon of a narrow window falls under the right group; then,
and only then, the Bookmark button is filled (`PageView.ribbonRight`), so the state never shows twice and never
disappears. Alternative: the core shortening the head to the space between the buttons — a new parameter of the page
style for one shell.

**4. Showing and hiding.** A `ReadingBar` object (new file `macos/Bar.swift`) owns the toolbar, the paper edges and the
state: hidden, shown, held (by the menu). It shows on pointer movement: a tracking area on the container with
`.mouseMoved`, `.activeInKeyWindow`, `.inVisibleRect` (no window-wide `acceptsMouseMovedEvents`). Showing unhides the
title bar's container view (the close button's superview's superview), then animates its `alphaValue` and the paper's
from 0 to 1 in 150 ms; hiding animates both to 0 in 300 ms and, on completion, hides the container so clicks reach the
page again. One non-repeating `Timer` of 2 s, restarted on each movement and not started while the pointer is inside
the bar's rectangle or the bar is held; when it fires over the page, `NSCursor.setHiddenUntilMouseMoves(true)`.
`Reader` hides the bar from a local event monitor on its window — any key, and a click outside the bar's strip — and
at the start of `run` for every command but the menu (panels, the help, a note, the view), and suppresses showing while
`panelOpen` or the view is open. Any key rather than only a page turn: simpler, and a key never wants the bar. Alternatives: Foliate's hover strip without a timer —
the maintainer chose the timer; showing on any key — breaks keyboard-first reading.

**5. The reading menu.** `ReaderMenu` gets a flag for macOS's reading bar: it does not add or animate its top bar,
and calls `bar.hold()` on open and `bar.release()` on close; a menu closed by a click or a key releases the bar
with an immediate fade, otherwise the 2 s timer runs from there. The bottom bar stays as it is. The menu's back and contents actions are the
toolbar's back and contents buttons.

**6. The bookmark state.** The bar asks the page where its ribbon hangs (nil when the page or spread has no bookmark)
when it shows, when a bookmark changes and when the page moves. Nothing new crosses the C ABI.

**7. Full screen.** The window delegate returns presentation options without `.autoHideToolbar`, so the toolbar stays
in the full-screen window and the bar's own show/hide applies; the system brings the window buttons with the menu bar.
If the system's toolbar window does not honour the container's alpha in full screen, the fallback is to let the
system auto-hide the toolbar at the top edge (Apple Books' own full-screen behaviour) — decided on the M1 in the tasks.

**8. Side and ABI.** Everything is in the macOS shell (`macos/Bar.swift`, `main.swift`, `Menu.swift`, `PageView.swift`).
The core, the C ABI and the GTK shell do not change.

## Risks / Trade-offs

- The title bar's container view is found through the window buttons' view hierarchy, which is not API. → Only
  `alphaValue` and `isHidden` are touched; if the hierarchy is not as expected the bar simply stays shown, as a
  standard toolbar. Checked on macOS 14; noted for macOS 15 and later.
- The groups' widths are estimates of the system's toolbar layout (back 56 points after the window buttons, four
  32-point buttons and 44 points of insets). → Checked on screenshots at 760 and 1100 points; a different system font
  size could shift them by a few points, which the 24-point fade absorbs.
- Panels open under the toolbar's area while the bar is hidden; if the bar could show over a panel the panel's top
  would lose clicks. → The bar does not show while a panel is open (spec).
- The window grows 28 points of page: the first page after this update may break at a different line than the user
  left it on. → Positions are `Locator`s; the line at the top stays the same.
- The fade runs on the render server; a main-thread probe cannot see its frames. → Measure with Instruments' Animation
  Hitches template (`xctrace`), and check that the page's `draw` is not called during a fade.

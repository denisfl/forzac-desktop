## 1. The window and the bar

- [x] 1.1 `macos/Bar.swift`: `ReadingBar` — the unified `NSToolbar` (back, flexible space, Contents, Text settings,
      Search, Bookmark; SF Symbols, icon only, not customisable), the paper fading under the two groups, tooltips and
      labels through `tr` (already in the catalogue)
- [x] 1.2 `main.swift`: the book window with `.fullSizeContentView`, transparent title bar, hidden title, no separator,
      the toolbar; the paper edges in the container above the page, below the panels; the toolbar's actions run
      `.contents`, `.settings`, `.search`, the bookmark toggle and the menu's back action
- [x] 1.3 The bookmark button outlined, filled only when the page's ribbon is under the right group; updated when the
      bar shows, a bookmark changes and the page moves

## 2. Showing and hiding

- [x] 2.1 Pointer tracking on the container (mouse moved, key window, visible rect): show (150 ms fade of the title
      bar's container and the paper, the container unhidden first), the 2 s timer restarted on movement, not running
      while the pointer is over the bar or the bar is held; hide (300 ms, then the container hidden); the pointer hidden
      until it moves when the timer hides the bar over the page
- [x] 2.2 Hide at once on a keyboard turn or scroll, a click on the page, opening a panel, the help, a note or the
      view; no showing while any of them is open
- [x] 2.3 Full screen: presentation options without auto-hiding the toolbar, the bar's behaviour checked there; the
      fallback of design decision 7 if the container's alpha does not apply in full screen
- [x] 2.4 `macos/build.sh` (PATH with `~/.rustup/toolchains/stable-aarch64-apple-darwin/bin` first) builds without
      warnings; `cargo test -p forzac-core` green (the catalogue test sees the new strings)

## 3. The reading menu

- [x] 3.1 `Menu.swift`: on macOS no top bar of its own; opening holds the reading bar, closing releases it (at once
      when a click or a key closed the menu); the bottom bar unchanged
- [x] 3.2 Screenshots of the menu open with the reading bar, in Light and Dark, for the maintainer's decision on one
      bar

## 4. Documents

- [x] 4.1 `docs/roadmap.md` (`macos-reading-bar` done), README if it describes the Mac window, the help if it needs a
      word on the bar

## 5. Verification on hardware

- [x] 5.1 M1 MacBook Pro, macOS 14: screenshots of a page with the bar hidden and shown (Light, Sepia, Dark), the
      bookmark button on a bookmarked page, the menu open, full screen with the bar; the bar's buttons do what `o`,
      `s`, `/`, `b` and back do; dragging the bar moves the window; keys never show the bar; the page's top line is the
      same before and after the update's first open
- [x] 5.2 No missed frames over the bar fading in and out, also during a page slide; the page's `draw` not called by a
      fade
- [x] 5.3 Idle: the bar faded and the pointer still, Forzac's CPU time unchanged over 10 s (`ps -o time`)
- [x] 5.4 Linux unchanged: no file under `gtk/` or `core/` changed except the Russian strings in `core/src/i18n.rs`

Checked 2026-10-20 on the M1 (macOS 14). 2.4: `macos/build.sh` without warnings, the core tests green. 5.1: screenshots
with `--mouse "move X Y"` (the pointer's move handed to the bar as its tracking area would; a synthetic move does not
reach a tracking area) and `screencapture -l` of the window: the bar hidden (the page to the top edge), shown at 760 and
1100 points (the head fading under the buttons at 760, the bookmark button filled there, outlined at 1100 with the
ribbon in sight), the menu in Light and Dark with one top bar. Real pointer moves over the test windows showed the bar
through the tracking area too. The maintainer checked by hand in the dev build (a copy of the state): the bar, its
buttons, the menu, full screen (`f`), the keys. Found there and fixed: with the system's keyboard navigation on, a
clicked toolbar button kept the keys (the page takes them back now). 5.2: a temporary display-link probe in the app
(Instruments' `xctrace` waited for developer-tools authorisation): 12 fades in 3 runs, two of them during a page slide,
0 frames later than 1.5 frame intervals, the longest interval 16.7 ms at 60 Hz. 5.3: 0.73 s → 0.73 s of CPU over 7 s
after the bar faded; 2.01 s → 2.01 s over 10 s idle in a normal run. 4.1: the roadmap; README and the help describe no
window chrome. Not done: the page's top line before and after the update's first open was not compared (the page
grows by 28 points; positions are Locators).

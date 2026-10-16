## 1. Core

- [x] 1.1 Contents and the note style in the core; the GTK shell uses them
- [x] 1.2 C ABI: settings, assets and font variations, contents, cover, authors, progress, notes, links, recent books,
      the continuous column; tests through the C functions

## 2. Settings and panels

- [x] 2.1 Swift split into files; fonts with variations; palette; interface fonts registered on first panel
- [x] 2.2 Settings applied (theme redraws, the rest re-lays out keeping the position), saved; appearance followed
- [x] 2.3 Settings sheet as in the design (keys and clicks)

## 3. Navigation

- [x] 3.1 Contents panel as in the design (filter, keys, jump remembered)
- [x] 3.2 Notes sheet; `Tab` through markers and links; clicks; links to places, notes and outside; `⌃O` back

## 4. Pages

- [x] 4.1 "Scroll": the continuous column in tiles; `m` switches keeping the top line; `j`/`k`, `⌃D`/`⌃U`
- [ ] 4.2 Trackpad: swipe turns a page; scrolling in "Scroll"

## 5. The app

- [x] 5.1 Document types; open from Finder, `⌘O`, recent documents; one window per book; recent books without a book
- [x] 5.2 Menus; help text; README
- [x] 5.3 The `.app` carries the fonts and licences

## 6. Verification

- [x] 6.1 fmt, clippy, tests; GTK still builds (Docker)
- [x] 6.2 On this Mac: every panel in Light, Sepia, Dark; settings keep the position; Scroll; a book from Finder

## 1. Core

- [ ] 1.1 Positions in a chapter layout: text before the chapter, per block, in the book; page progress
- [ ] 1.2 Running head (not on a chapter's first page) and the foot "n / N · p%", shortened with "…"
- [ ] 1.3 `ChapterLayout::height`; a note as a one-chapter book
- [ ] 1.4 Tests: progress per page, head and foot, a note's layout

## 2. GTK

- [ ] 2.1 Drawing from page contents shared by the page and the note view; areas cached per page
- [ ] 2.2 Shortcuts on the page view; panels take the focus
- [ ] 2.3 Contents panel: entries with positions, current selected, j/k, Enter, `/` filter, Esc, `o`
- [ ] 2.4 Tab / Shift+Tab focus over areas, highlight, Enter opens, click opens
- [ ] 2.5 Note panel typeset by the core; Esc/Enter closes
- [ ] 2.6 Links: places with the back stack (`Ctrl+O`), notes, external through the browser
- [ ] 2.7 Wheel and touchpad turn pages; help text with the new keys

## 3. Verification

- [ ] 3.1 fmt, clippy, tests on macOS and Linux
- [ ] 3.2 Raspberry Pi 5: scripted session (contents, a note, a link and back) with screenshots; timings; turns miss no
      frames

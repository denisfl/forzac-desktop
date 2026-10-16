## 1. Core

- [x] 1.1 Positions in a chapter layout: text before the chapter, per block, in the book; page progress
- [x] 1.2 Running head (not on a chapter's first page) and the foot "n / N · p%", shortened with "…"
- [x] 1.3 `ChapterLayout::height`; a note as a one-chapter book
- [x] 1.4 Tests: progress per page, head and foot, a note's layout

## 2. GTK

- [x] 2.1 Drawing from page contents shared by the page and the note view; areas cached per page
- [x] 2.2 Shortcuts on the page view; panels take the focus
- [x] 2.3 Contents panel: entries with positions, current selected, j/k, Enter, `/` filter, Esc, `o`
- [x] 2.4 Tab / Shift+Tab focus over areas, highlight, Enter opens, click opens
- [x] 2.5 Note panel typeset by the core; Esc/Enter closes
- [x] 2.6 Links: places with the back stack (`Ctrl+O`), notes, external through the browser
- [x] 2.7 Wheel and touchpad turn pages; help text with the new keys

## 3. Verification

- [x] 3.1 fmt, clippy, tests on macOS and Linux
- [x] 3.2 Raspberry Pi 5: scripted session (contents, a note, a link and back) with screenshots; timings; turns miss no
      frames that the previous version did not miss (the wheel, clicks and the browser are not checked: no pointer or
      browser in the headless session)
- [x] 3.3 Found on the way: EPUB list items of paragraphs kept their marker on a line of its own; fixed in the XHTML
      reader

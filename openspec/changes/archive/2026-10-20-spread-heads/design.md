## Context

The core renders a page with its head and foot (`ChapterLayout::render_numbered`); on a spread each half of the window
is typeset as a page of half the width (`Settings::page_style`), and every chapter starts on the left page, so a
spread's left page is an even page of its chapter. The ribbon is placed by each shell (`PageView.updateRibbon`,
`PageView::ribbon_dx`) over the page holding the spread's first bookmark.

## Decisions

**1. The page style says it is a spread's page.** `PageStyle` gets `spread: bool`, set by `Settings::page_style` when
the window shows a spread (false in `PageStyle::column`, notes and tables). The style is already compared to tell
layouts and the book's page count apart, so a half-window page and a spread page stay different. Alternative: a
parameter to `render_numbered` — every caller (both shells, the count) would have to pass it.

**2. The book's title for the left head.** `ChapterLayout` keeps the book's title next to the chapter's
(`paginate_from`); `render_numbered` uses it on an even page index above 0 when `style.spread`, the chapter's title
otherwise. An empty book title leaves the left head empty.

**3. The ribbon over the right page.** On a spread both shells place the ribbon at the right page's offset; on a single
page nothing changes. The corner click already belongs to the right page.

**4. Side and ABI.** Core and both shells; nothing new crosses the C ABI.

## Risks / Trade-offs

- A chapter whose page count is odd ends on a left page with the book's title over it and empty paper beside: as in a
  printed book's verso, acceptable.

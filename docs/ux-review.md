# UX review of other readers

What other e-book readers do well, what Forzac should borrow, and what it should not. Asked for by the maintainer on
2026-10-20 ("study other tools' UX and take what is best and convenient; our job is to make a good solution"). The
maintainer decides each "take"; the chosen items go to [roadmap.md](roadmap.md).

## How this was done

| Reader | Version | How |
|---|---|---|
| Apple Books | 6.4 on macOS 14 (this Mac) | Apple's Books for Mac User Guide; the app's own English string tables and presets (`/System/Applications/Books.app`, its frameworks' `Localizable.loctable`, `FontPresets.plist`); the maintainer's screenshots. Driving the app by hand was not possible: computer use of Books is blocked by policy on this Mac. |
| Kindle for Mac | 7.x (the 2023 Catalyst app; 7.69, October 2026) | Amazon's help pages, the App Store listing, reviews (mjtsai, GoodeReader, the-ebook-reader). Amazon's key list may describe the older "Classic" app. |
| Foliate | 3.0–3.3 (GTK 4, libadwaita) | Source code (`foliate` gtk4 branch, `foliate-js`), release notes, issues. |
| KOReader | master, 2026 | Source code, the user guide, the wiki, forums. |
| Readest | 2026 (Tauri, on foliate-js) | Its documentation, source code, issues. |
| Thorium Reader | 3.5.1 | Source code, documentation, release notes, issues. |
| Bookmate (Yandex Books since 2024) | Android 6.85 | Reviews, store listings, news; the official help could not be reached, so this column is thin. |

The yardstick is Forzac's principle ([CLAUDE.md](../CLAUDE.md)): few settings with good defaults, presets instead of
sliders, keyboard first, the typography of a good printed book, smoothness over features (nothing drawn while idle), no
cloud. "Have" means Forzac already does it (the spec is named).

Each solution has an id for the decision: **W** reading window, **P** page turning and moving around, **C** contents
and search, **S** settings, **H** highlights and notes, **B** bookmarks, **D** dictionary, **L** library, **O** onboarding
and help.

## Reading window

Every desktop reader studied hides its chrome while reading, but in different ways:

- **Apple Books** shows its controls "when the pointer moves to the top of the book": the window's buttons, the book's
  title in the middle, and icon buttons — Table of Contents, Notes, Appearance ("Aa"), Search, Bookmark (filled on a
  bookmarked page) — in a toolbar drawn over the page, under a transparent title bar. Left alone it fades away and the
  window is only the page. Edge arrows appear when the pointer nears a side.
- **Foliate 3** puts the header bar and a bottom bar in crossfading revealers over the page. A bar shows while the
  pointer is over its strip and hides as soon as it leaves — no timer — and stays while one of its popovers is open;
  a click on the page hides both. "Autohide Cursor" hides the pointer after 1 s without movement.
- **Readest** does the same with invisible strips at the top and bottom edges (300 ms fade); keyboard focus shows a bar,
  `Esc` hides it; the macOS window buttons hide 100 ms after the pointer leaves.
- **Kindle** shows its bars on a click near the top; when its menu opens the page shrinks to show whole, with slivers of
  the neighbouring pages. **Thorium**'s toolbar never hides (only a "Zen mode" does), and its most upvoted open issue
  is that the page number shows only on hover.
- **Forzac** has the web reader's reading menu (`reader-menu`: a click in the middle third or `Return`), a running head
  with the chapter's title and a foot with "37 of 212" always on the page (`reader-navigation`). On macOS the standard
  title bar is always there, 28 pt of chrome above the page.

| Id | Solution | Seen in | Why it is good | Verdict |
|---|---|---|---|---|
| W1 | Content under a transparent title bar; a top bar with the window buttons, the title and icon buttons that appears with the pointer and fades away | Apple Books, Foliate, Readest | The page fills the window like a sheet of paper; the controls are one pointer move away and need no learning; a Mac reader looks like a Mac app | **Take** — first change, macOS only (`macos-reading-bar`, below) |
| W2 | The pointer hides with the bar | Foliate (1 s), Readest | A pointer resting on the text is the last piece of chrome; no setting needed | **Take** — inside W1 |
| W3 | Time left from the reader's own speed: pages left × the average time a page took in this book, a page counting at most 120 s; a fixed rate until there is enough data | Kindle, KOReader (Foliate and Forzac: a fixed rate) | The estimate becomes true for this reader and this book; nothing to set. Forzac's "N min left in chapter" assumes 1000 characters a minute for everyone | **Take** — small, core |
| W4 | "Time left in book" next to "in chapter" | Foliate, Kindle, KOReader | Answers "will I finish it on this flight?" | **Take** with W3, in the menu's hint line |
| W5 | One footer slot that cycles on click (location, time left in chapter, in book, none) | Kindle, KOReader | Little chrome, the reader picks | **Don't take** — a hidden state and a setting in disguise; Forzac's foot already shows the page, the menu the time |
| W6 | The page shrinks when the menu opens | Kindle | The menu never covers text | **Don't take** — the page moving under the pointer is restless; `reader-menu` keeps the page still |
| W7 | A quiet page number always on the page | Foliate, Readest; missing in Thorium (#3718) | The most asked-for orientation | **Have** (`reader-navigation`) |
| W8 | A clock while reading | Kindle, KOReader | Useful in full screen | **Don't take** — the menu bar has one; one more thing drawn every minute |
| W9 | Edge arrows on hover to turn pages | Apple Books | Discoverable for mouse users | **Don't take** — the side thirds already turn pages on click, as taps on Android; arrows would appear and fade on every pass of the pointer |

## Page turning and moving around

| Id | Solution | Seen in | Why it is good | Verdict |
|---|---|---|---|---|
| P1 | A jump history with Forward as well as Back: only jumps (contents, links, search, slider) make entries, page turns do not | Foliate (`Alt+←/→`, mouse buttons 4/5), Readest (`Shift+←/→`), Thorium | After going back, the way forward again is not lost; Back stays meaningful | **Take** — Forzac has Back (`Ctrl+O`, 50 places, the pill); add Forward as Vim's `Ctrl+I`, `⌘]` on macOS, and the mouse's back/forward buttons |
| P2 | Go to a page by typing its number into the search field | Apple Books ("Type a word or page number"); Kindle `⌘G`, Thorium a separate field | No new panel: the field that is already there understands a number | **Take** — small; once the book's pages are counted (`reader-menu`), a number alone offers "Page N" as the first row |
| P3 | Follow the system's "Reduce motion" | Foliate and Thorium have their own checkbox | Animations off for those who need it, with no setting in Forzac | **Take** — small; "Slide" becomes a cross-fade or a cut, the menu and panels appear without sliding |
| P4 | Mac shortcuts as in Books: `⌥←`/`⌥→` previous/next chapter, `⌥↑`/`⌥↓` start/end of the chapter | Apple Books | What Mac readers' fingers already know; `[`/`]` stay | **Take** — tiny, macOS menus |
| P5 | Several page-turn animations (curl, push, none) | Apple Books, Kindle, Readest | Taste | **Don't take** — one excellent "Slide" (`page-turning`) |
| P6 | Number keys jump to 0 %, 10 %, … 100 % | KOReader | Fast for e-ink | **Don't take** — `0` is the default text size; the slider and `g`/`G` cover it |
| P7 | Two-finger swipe, one page per swipe; wheel turns pages; the side thirds click | Apple Books, Foliate; Thorium lacks it (#1436) | Natural on a Mac | **Have** (`macos-app`, `reader-navigation`) |
| P8 | Two pages when the window is wide | Apple Books, Foliate, Readest | A book's spread | **Have** (`page-turning`, 0.11.0) |

## Contents and search

| Id | Solution | Seen in | Why it is good | Verdict |
|---|---|---|---|---|
| C1 | Contents with the share read in its header ("Contents • 37 %"), the current entry marked, pages at the right | Apple Books, Readest | Where am I in the whole book | **Have** (`reader-navigation`) |
| C2 | Search results with a snippet, the match in bold, the section and the page | Foliate, Readest, Apple Books | Choosing the right match without jumping | **Have** (`book-search`) |
| C3 | Search the selected text | Apple Books (Control-click › Search), Foliate (`Ctrl+F` with a selection) | "Where else is this name?" in one step | **Take** — tiny: `/` with text selected searches it |
| C4 | "Return to search" / ↺ back to where the search started | Apple Books, KOReader | Trying matches without getting lost | **Have** — `Ctrl+O` and the pill after a search jump |
| C5 | Search options (whole words, case, regular expressions, scope) | Foliate, Readest, KOReader | Power | **Don't take** — Forzac ignores case and ё/е, that is enough for reading |
| C6 | Thumbnails, a "book map", a page browser | Apple Books (picture books), KOReader | Visual books | **Don't take** — text books; the slider with chapter ticks is the map |
| C7 | Recent searches | Apple Books | Repeat a search | **Don't take** — rarely needed in one book |

## Settings

Apple Books' "Themes & Settings" is the closest to Forzac's idea and still goes further than Forzac wants: six themes
(Original, Quiet, Paper, Bold, Calm, Focus) over Light/Dark/Automatic, twelve text sizes, and a "Customize" switch that
opens a font list, Bold Text, sliders for line, character and word spacing, Justify Text and Allow Multiple Columns.
Kindle names layout presets (Compact, Standard, Large, Low Vision) and saves custom themes. Foliate gives each of nine
themes a light and a dark variant chosen by the system. Thorium's "Save / Apply / Reset" of a preset confuses its
users (#3283); Readest's single "Global Settings" switch confuses them less. KOReader is the cautionary tale: settings
are per book by default, with profiles, a quick menu and a menu search because the tree is too deep to find things in.

| Id | Solution | Seen in | Why it is good | Verdict |
|---|---|---|---|---|
| S1 | One set of settings for every book, no per-book overrides | Readest (default), Foliate | One model, nothing to explain | **Have** (`reader-settings`) — keep it |
| S2 | Light or dark from the system by default | Apple Books (Automatic), Foliate, Thorium 3 | Right at night without a key | **Have** (`reader-settings`) |
| S3 | Few fixed steps instead of sliders | Thorium, KOReader's presets | Each step is a good one | **Have** — seven sizes, three spacings, three margins |
| S4 | Sliders for spacing, Bold Text, per-theme customisation | Apple Books | Fine control | **Don't take** — the opposite of presets |
| S5 | Named layout presets (Compact, Standard, Large, Low Vision) | Kindle | One choice instead of three | **Don't take** for now — Forzac has three layout settings only; worth a look if a "large print" need appears |
| S6 | `⌘+`, `⌘−`, `⌘0` for the text size | Apple Books, Kindle | Mac habit | **Have** (View menu) |

## Highlights and notes

| Id | Solution | Seen in | Why it is good | Verdict |
|---|---|---|---|---|
| H1 | A note on a highlight: written in a small sheet, shown in the highlights list under the quote, exported with it | All of them: Apple Books, Kindle, Foliate, KOReader, Readest, Thorium | A thought next to the quote is the point of highlighting for many readers | **Take** — medium; keyboard: `n` in the selection or on a marker opens the note, `Esc` saves |
| H2 | Save the highlights to a Markdown file | Foliate (Markdown, HTML, JSON, Org), Readest, KOReader; Kindle for Mac lost it and is criticised for it, Apple Books never had it | Highlights outlive the app and go into notes apps; files, not a cloud | **Take** — small; the panel's text (already "> " quotes with sections) saved as `Title.md`, notes included (H1) |
| H3 | Five to nine colours, underline and squiggle styles | Apple Books (yellow, green, blue, pink, purple, underline), Foliate, KOReader, Thorium | Coding by colour | **Don't take** — one ochre marker as on Android; colours are a setting in every highlight |
| H4 | Export all books' highlights at once | KOReader, Readest | Useful with a big library | **Later** — with the library work (L1) |
| H5 | Edit a highlight's text, highlight every occurrence, freehand drawing | KOReader, Readest | Niche | **Don't take** |
| H6 | Other readers' popular highlights, social quotes | Kindle, Bookmate | Social | **Don't take** — offline, private |

## Bookmarks

| Id | Solution | Seen in | Why it is good | Verdict |
|---|---|---|---|---|
| B1 | A ribbon on a bookmarked page, the toolbar button filled, `⌘D` | Apple Books, Foliate, Kindle | Visible state, one key | **Have** (`bookmarks`: `b`, `⌘D`, the ribbon, a click on the corner); W1 adds the filled button |
| B2 | Named or tagged bookmarks | Readest, Thorium | Organising | **Don't take** — the list shows the text at the bookmark |

## Dictionary and lookup

| Id | Solution | Seen in | Why it is good | Verdict |
|---|---|---|---|---|
| D1 | The system's Look Up on macOS: a word under a three-finger tap or force click, or `⌃⌘D`, shows the dictionary popover | Apple Books ("Look Up"), every AppKit text view | Offline, in the reader's languages, maintained by Apple, no settings; one AppKit call (`showDefinition(for:at:)`) | **Take** — small, macOS only; "dictionaries" is in the roadmap's "Not planned for 1.0", so this needs the maintainer's yes |
| D2 | Bundled offline dictionaries (StarDict, MDict) | KOReader, Readest | Works everywhere | **Don't take** — large files, licences, a dictionary manager |
| D3 | Wikipedia, Google Translate, AI questions about the book | Foliate, KOReader, Readest, Kindle | Convenient | **Don't take** — online |

## Library

The maintainer's direction is a Calibre-like library over the Books folder. What the others offer:

- **Apple Books**: a sidebar with Home ("Continue"), All, Want to Read, Finished (books land there by themselves when
  read to the end, as a timeline), and the reader's own collections; Sort By Recent, Title, Author or Manually;
  "Import…" with "Importing: N of M. Failed: K"; reading goals and a year in review.
- **Readest**: shelves with filters, groups by author, series or tag, many sort keys (including time remaining), editing
  of title, author, series and tags kept beside the file, which is never modified.
- **KOReader**: collections, a status per book (reading, on hold, finished), Calibre's wireless connection and a search
  over Calibre's metadata. **Foliate**: only "recent", and bulk import is its top library request (#730).

| Id | Solution | Seen in | Why it is good | Verdict |
|---|---|---|---|---|
| L1 | Collections (shelves) the reader makes, plus automatic ones: Finished, and by author and series | Apple Books, KOReader, Readest | A library past 50 books needs grouping; automatic groups cost the reader nothing | **Take** — in the Calibre-like library work |
| L2 | Sorting by Recent (default), Title, Author | Apple Books, Readest | Finding by name in a big library | **Take** — with L1 |
| L3 | Editing title, authors, series without touching the file | Readest, Calibre | Bad metadata is common in FB2 and EPUB | **Take** — with L1, kept in the state file |
| L4 | A folder that is the library, books copied in on open | Readest ("read in place"), Thorium (copies) | Files are the user's | **Have** (`books-folder`) |
| L5 | Reading goals, streaks, a year in review, statistics | Apple Books, KOReader, Bookmate | Motivation for some | **Don't take** — gamification and settings; maybe never |
| L6 | Search across the text of all books | Readest | Power | **Don't take** — an index to maintain |

## Full screen, onboarding and help

| Id | Solution | Seen in | Why it is good | Verdict |
|---|---|---|---|---|
| O1 | The same disappearing bar in full screen | Apple Books, Foliate, Readest | One behaviour in both | **Take** — part of W1 |
| O2 | Keep the display awake while reading, for a while after the last input | Apple Books ("Extend by 10 minutes while reading") | Reading is input-free for minutes | **Have** (`reader-window`) |
| O3 | A list of keys on `?` | Foliate, Readest, Thorium (searchable, editable) | Keyboard first needs it | **Have** (`help`) — no editable keys |
| O4 | A command palette (`⌘⇧P`) | Readest | Everything findable by name | **Don't take** — fewer commands than a palette justifies |
| O5 | Coach marks on first run | Apple Books | Teaches gestures | **Don't take** — the empty library and `?` are enough |
| O6 | "Earlier in the book" recap when returning after a break | Bookmate (AI) | Gets back into the story | **Don't take** — online AI |

## Summary: the "take" candidates

Ordered by value for the effort, for the maintainer to choose from:

| # | Ids | What | Size |
|---|---|---|---|
| 1 | W1, W2, O1, B1 | `macos-reading-bar`: the Apple Books reading window on macOS | 3–4 d |
| 2 | P1 | Forward in the jump history (`Ctrl+I`, `⌘]`, mouse buttons) | 1 d |
| 3 | P2, C3 | The search field takes a page number; `/` searches the selection | 1 d |
| 4 | P3 | Follow the system's "Reduce motion" | 1 d |
| 5 | P4 | `⌥` arrows for chapters on macOS | ½ d |
| 6 | W3, W4 | Time left from the reader's own speed, in the chapter and the book | 2–3 d |
| 7 | D1 | Look Up on macOS (needs a yes: dictionaries are "not planned") | 1 d |
| 8 | H1, H2 | Notes on highlights; highlights saved as Markdown | 1 w |
| 9 | L1, L2, L3 | Collections, sorting, metadata editing — the Calibre-like library | 2–3 w |

## The first change: `macos-reading-bar`

Refined with what the review found:

- Content under a transparent title bar (`fullSizeContentView`, the title hidden). The page's top margin (56 px) holds
  the running head; the bar (52 pt) covers only that margin, never text.
- The bar: the window buttons at the left, the book's title in the middle, at the right Contents (`o`), Aa (`s`),
  Search (`/`), Bookmark (`b`, filled on a bookmarked page) — Lucide icons, tooltips with their keys.
- When it shows: the pointer moves (anywhere in the window — as in Books, so a mouse user finds it without knowing
  where to look). When it goes: about 2 s after the pointer stops, unless the pointer is over the bar; at once on a
  page turn from the keyboard or a click on the page (keyboard readers never see chrome they did not ask for).
  Foliate and Readest hide only when the pointer leaves a strip; a timer is kinder to a mouse left anywhere on the page.
- The pointer hides with the bar (W2), and comes back when it moves.
- Fades in and out on Core Animation (the compositor), no redraw of the page; nothing runs while idle.
- In full screen the same; on Linux nothing changes.
- Open question for the design: how it lives with the reading menu (`reader-menu`), whose own top bar has Back to
  library, the section and the contents.

## Sources

Apple Books
- Keyboard shortcuts and gestures: https://support.apple.com/guide/books/keyboard-shortcuts-and-gestures-ibk0a716cf4-9a70-485b-9835-72e3c7ed6c29/mac
- Read books (Books 6, macOS 14): https://support.apple.com/en-us/guide/books/ibks5f526382/6.0/mac/14.0
- Change a book's appearance: https://support.apple.com/guide/books/ibks8923126d/mac
- Highlight passages and add notes: https://support.apple.com/guide/books/ibks3975f128/mac
- Organise with collections: https://support.apple.com/guide/books/ibks33867842/mac
- The app's string tables: `/System/Applications/Books.app/Contents/Resources/Localizable.loctable` and the
  `Localizable.loctable` of its `BookEPUB`, `BooksUI` and `AEBookPlugins` frameworks; `AEBookPlugins.framework/…/FontPresets.plist`

Kindle for Mac
- Kindle for Mac 7.0 review and complaints: https://mjtsai.com/blog/2023/11/15/kindle-for-mac-7-0/
- New Kindle app for the Mac: https://goodereader.com/blog/kindle/amazon-releases-new-kindle-app-for-the-mac
- Reading progress options: https://goodereader.com/blog/?p=262073
- Page view like the apps: https://blog.the-ebook-reader.com/2025/06/26/amazon-changing-page-view-on-kindles-to-look-more-like-kindle-apps/
- New Kindle for Mac released: https://blog.the-ebook-reader.com/2023/09/29/new-kindle-for-mac-app-released
- App Store listing: https://apps.apple.com/us/app/amazon-kindle/id302584613?platform=mac

Foliate
- Releases: https://github.com/johnfactotum/foliate/releases
- Book viewer (autohide bars): https://raw.githubusercontent.com/johnfactotum/foliate/gtk4/src/book-viewer.js
- Navbar (slider with section ticks, location popover): https://raw.githubusercontent.com/johnfactotum/foliate/gtk4/src/ui/navbar.ui
- Cursor autohide, history, time left: https://raw.githubusercontent.com/johnfactotum/foliate-js/main/view.js
- Annotations and export: https://raw.githubusercontent.com/johnfactotum/foliate/gtk4/src/annotations.js
- Themes: https://raw.githubusercontent.com/johnfactotum/foliate/gtk4/src/themes.js
- Bulk import request: https://github.com/johnfactotum/foliate/issues/730

KOReader
- User guide: https://koreader.rocks/user_guide/
- Status bar: https://raw.githubusercontent.com/koreader/koreader/master/frontend/apps/reader/modules/readerfooter.lua
- Reading statistics (time left from the reader's speed): https://raw.githubusercontent.com/koreader/koreader/master/plugins/statistics.koplugin/main.lua
- Search: https://raw.githubusercontent.com/koreader/koreader/master/frontend/apps/reader/modules/readersearch.lua
- Export of highlights: https://raw.githubusercontent.com/koreader/koreader/master/plugins/exporter.koplugin/main.lua
- Dictionaries: https://github.com/koreader/koreader/wiki/Dictionary-support
- Calibre: https://github.com/koreader/koreader/wiki/calibre
- On its settings: https://lwn.net/Articles/891494

Readest
- Reading: https://readest.com/docs/reading
- Customisation: https://readest.com/docs/customization
- Keys: https://readest.com/docs/advanced
- Library: https://readest.com/docs/library
- Source: https://github.com/readest/readest

Thorium Reader
- Releases: https://github.com/edrlab/thorium-reader/releases
- Default keys: https://thorium.edrlab.org/en/th3/400_resources/402_keyboard-default/
- FAQ: https://thorium.edrlab.org/en/th3/400_resources/430_faq
- Wheel and trackpad over the page: https://github.com/edrlab/thorium-reader/issues/1436
- Page number only on hover: https://github.com/edrlab/thorium-reader/issues/3718
- Default settings for all books: https://github.com/edrlab/thorium-reader/issues/3283

Bookmate / Yandex Books
- Overview and shelves: https://t-j.ru/book-mate/
- Readers compared: https://t-j.ru/list/best-chitalki/
- RuStore listing and reviews: https://www.rustore.ru/catalog/app/ru.plus.bookmate
- "Earlier in the book": https://habr.com/ru/post/975874

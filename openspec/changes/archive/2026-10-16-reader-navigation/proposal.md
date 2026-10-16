## Why

The spike can only turn pages and jump between chapters. Reading a whole book needs the table of contents,
footnotes, links that can be followed and returned from, and a sense of where you are — all from the keyboard.

## What Changes

- Contents panel on `o`: chapters with nesting and page numbers in the current layout, the current chapter selected;
  `j`/`k` or arrows to move, `Enter` to jump, `/` to filter, `Esc` to close.
- Footnotes: markers on the page can be activated; the note opens in a panel over the page (`Esc` closes). Keyboard:
  `Tab` cycles the active markers and links of the page, `Enter` opens.
- Internal links jump within the book and remember where they came from; `Ctrl+O` returns (as in Vim); external links
  open in the browser.
- Progress: chapter title and position in the book shown unobtrusively; `gg`/`G` keep their chapter meaning.
- Mouse: click on markers and links; scroll wheel turns pages.

## Capabilities

### New Capabilities
- `reader-navigation`: contents, notes, links, back navigation, progress, the keys for each.

### Modified Capabilities
<!-- None. -->

## Impact

Core: contents with Locators, active areas per page (from `typesetting-complete`). Shells: panels and key handling.

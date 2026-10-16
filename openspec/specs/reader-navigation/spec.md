# reader-navigation Specification

## Purpose
How a reader moves through a book beyond turning pages: the contents, footnotes, links and the way back, and how the
page shows where the reader is. Everything is reachable from the keyboard; the mouse is a second way.

## Requirements

### Requirement: Contents
`o` SHALL open the contents in a panel over the page: the book's titled sections with their nesting and their position
in the book (in percent), the entry of the current place selected. `j`/`k` or `↓`/`↑` move the selection, `Enter`
jumps to the entry and closes the panel, `/` filters the entries by the typed text (ignoring case and the difference
between "ё" and "е"), `Esc` clears the filter or closes the panel, `o` closes it. A book without contents shows its
chapters. While the panel is open, keys do not turn pages.

#### Scenario: Jumping to a chapter
- **WHEN** the reader presses `o`, types `/` and "наука", and presses `Enter`
- **THEN** the book opens at the first matching section and the panel closes

### Requirement: Active markers and links
The note markers and links of the current page SHALL be reachable from the keyboard: `Tab` and `Shift+Tab` cycle
through them in reading order, highlighting the focused one; `Enter` opens it; `Esc` or turning the page removes the
focus. Without focus, `Enter` turns the page as before. A click on a marker or link opens it.

#### Scenario: A footnote from the keyboard
- **WHEN** a page has two note markers and a link and the reader presses `Tab` twice and `Enter`
- **THEN** the second marker's note opens

### Requirement: Notes
A note SHALL open in a panel over the lower part of the page, typeset like the book (same fonts, hyphenation and
justification) in the current theme, scrollable when long. `Esc` or `Enter` closes it; the reading position does not
change.

#### Scenario: Reading a footnote
- **WHEN** the reader opens a note marker
- **THEN** the note's title and text appear over the page and `Esc` returns to the unchanged page

### Requirement: Links and the way back
A link to a place in the book SHALL go to the page where the target starts; a link to a note opens the note; a link
outside the book opens in the default browser. Every jump (a link, a contents entry) SHALL remember where it started:
`Ctrl+O` returns to the place before the last jump, repeatedly, up to 50 places.

#### Scenario: Following a cross-reference
- **WHEN** the reader follows "see Chapter 10" from page 5 of chapter 1 and then presses `Ctrl+O`
- **THEN** the book shows chapter 10, then page 5 of chapter 1 again

### Requirement: Where the reader is
Every page except the first page of a chapter SHALL show the chapter's title as a running head in the secondary ink,
shortened with "…" when it does not fit. The foot of the page SHALL show the page within the chapter and the position
in the book: "4 / 22 · 37%".

#### Scenario: Middle of a book
- **WHEN** the reader is on page 4 of the 22 pages of a chapter that ends at 40% of the book
- **THEN** the head shows the chapter title and the foot "4 / 22 · 37%" (or the actual share)

### Requirement: Mouse and touchpad
A scroll wheel step SHALL turn one page (down: forward); one touchpad swipe SHALL turn at most one page. Clicks on
markers and links open them.

#### Scenario: Scrolling with a wheel
- **WHEN** the reader turns the wheel down three steps
- **THEN** three pages are turned forward

### Requirement: No cost while reading
Opening the contents SHALL take at most 50 ms for a book of 2000 sections on a Raspberry Pi 5; a note SHALL open
within 30 ms; the running head and foot SHALL not change the time to typeset a chapter by more than 5%, and page turns
at 120 Hz SHALL miss no frames.

#### Scenario: A long book
- **WHEN** the contents of a 2000-chapter book are opened on a Raspberry Pi 5
- **THEN** the panel appears within 50 ms

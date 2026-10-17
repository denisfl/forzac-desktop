# Spec Delta

## Purpose
Selecting text on the page with the mouse or the keyboard, copying it, and keeping highlights (the Android app's
quotes): marked passages shown under the text, listed in the contents panel and exported, kept by text position so
they survive re-layout and restarts.

## ADDED Requirements

### Requirement: Words
Selection SHALL snap to words: a word is a run of characters between white space, its punctuation included ("верно."
is one word); a word hyphenated across two lines is one word. A selection is the text from the start of its first word
to the end of its last, in reading order, across blocks, pages and chapters; note markers and table cells are text like
any other, images are not.

#### Scenario: A hyphenated word
- **WHEN** the reader selects the first half of "на-укой", hyphenated at a line's end
- **THEN** the selection covers the whole word "наукой" on both lines

### Requirement: Selecting with the mouse
Pressing on the text and dragging SHALL select from the word under the press to the word under the pointer; a double
click SHALL select the word, a triple click the paragraph (a table row: the row) — in "Slide" in the middle third of
the page, since a click in a side third turns the page at once and quick clicks there keep turning; `Shift`+click SHALL
extend the selection to the clicked word. The selection's ends SHALL show handles (a 2 px bar the line's height and a knob, in the
link ink, as on Android); dragging a handle moves that end. In "Slide", holding the pointer for 0.5 s within 24 px of
the window's right edge or below the text turns the page forward, near the left edge or above the text back, and the
selection goes on on the new page; in "Scroll", dragging above or below the window scrolls the column, faster the
further out. A press that does not move does not select; a click away from the selection, its bar and its handles
SHALL clear it, doing nothing else.

#### Scenario: A sentence by drag
- **WHEN** the reader presses on "Протеин," and drags to "верно." two lines below
- **THEN** the selection covers both words and everything between, and the bar appears above it

#### Scenario: Across a page
- **WHEN** the reader drags an end to the right edge of the window and holds it there
- **THEN** after half a second the next page slides in and the selection continues on it

### Requirement: Selecting from the keyboard
`v` SHALL start the selection mode with a word cursor on the first word of the page (in "Scroll", the first word below
the top margin), boxed in the link ink. In the mode `h`/`←` and `l`/`→` SHALL move the cursor to the previous and next
word, `j`/`↓` and `k`/`↑` to the word on the next and previous line nearest the cursor's middle; moving past the page's
first or last word turns the page (in "Scroll" scrolls the column by a line) and the cursor goes on. `v` SHALL anchor the
selection at the cursor's word; after that every move selects from the anchor to the cursor, either way round; `v` again
or `Esc` drops the anchor; `o` swaps the anchor and the cursor. `Esc` without an anchor leaves the mode. A mouse
selection is the same state: its start is the anchor and its end the cursor, so the keys adjust it. While the mode is
on, the other page keys do nothing.

#### Scenario: Two sentences from the keyboard
- **WHEN** the reader presses `v`, `j` twice, `v`, then `l` until the end of the next sentence, and `Enter`
- **THEN** the text from the cursor's word to the sentence's end becomes a highlight and the mode ends

### Requirement: The selection bar and copying
A selection SHALL show a bar over it as the Android design's: a rounded bar in the ink with the paper's colour for its
labels, Golos Text 15 medium, 14 px above the selection (below it when there is no room above), the items "Highlight",
"Copy" and, on macOS, "Share", separated by thin rules. A click on an item or its key acts: `Enter` highlights, `c`,
`Ctrl+C` (`⌘C` on macOS) copies, `s` shares (macOS). Copying SHALL put the selected text on the clipboard: blocks on
lines of their own, without soft hyphens and note markers, no-break spaces as spaces. Sharing SHALL open the system
share menu at the bar with «the text» followed by a blank line and "— Author, «Title»" (Android's form). Highlighting,
copying or sharing ends the selection. Linux has no "Share" (no system share menu).

#### Scenario: Copying a phrase
- **WHEN** the reader selects a phrase with a hyphenated word and a note marker in it and presses `Ctrl+C`
- **THEN** the clipboard holds the phrase with the word whole and without the marker

### Requirement: Highlights
"Highlight" SHALL keep the selection as a highlight: its start and end Locators, kept in the state file with the
bookmarks' rules (`bookmarks`), belonging to the book by its content. A highlight SHALL be drawn under the text as
Android's marker: rounded bands along its lines, 2 px wider on each side, in ochre `#C49A52` at 32 % (30 % on Dark,
Wave and Forest). A new highlight SHALL appear with a stroke along its lines, left to right, line after line, in 90 ms a
line (at least 180 ms, at most 600 ms). A click on a marker, or `Enter` on a marker focused with `Tab`, SHALL show the
bar with "Delete", "Copy" and, on macOS, "Share" (keys `d`, `c`, `s`; `Esc` closes it). Highlights SHALL survive a
change of text size, font, spacing, margins, the page mode and a restart, and stay on the same words.

#### Scenario: A larger text size
- **WHEN** the reader highlights a phrase and makes the text larger
- **THEN** the marker lies under the same words on their new lines

#### Scenario: Deleting from the page
- **WHEN** the reader clicks a marker and then "Delete"
- **THEN** the marker goes, and the highlight is gone from the list

### Requirement: The highlights list and export
The contents panel SHALL have a "Highlights" tab after "Bookmarks": the book's highlights in reading order, each with a
3 px ochre rule on its left, its text (PT Serif 16/23, at most six lines) and below it "section · N%". `j`/`k` move,
`Enter` (or a double click) SHALL go to the page where it starts, remembered for `Ctrl+O` like a contents jump, `d`
SHALL delete it, `c` copy its text. `Ctrl+C` (`⌘C`) SHALL copy all the book's highlights as Android exports them: the
title and authors joined by " — ", a blank line, then each highlight with every line prefixed "> ", its section on a
line "— Section" when it has one, highlights apart by a blank line; on macOS a share button in the panel's header
shares the same text. Without highlights the tab SHALL say "No highlights yet" and "Select text with the mouse, or press
v on a page, then choose Highlight."

#### Scenario: Copying all highlights
- **WHEN** a book has two highlights in chapters 3 and 7 and the reader presses `Ctrl+C` on the "Highlights" tab
- **THEN** the clipboard holds the title line, then "> " lines of each highlight with "— Глава 3" and "— Глава 7"

### Requirement: Cost
Dragging a selection SHALL redraw no more than the page or the column's tiles on screen and miss no frame at 120 Hz on
a Raspberry Pi 5; a highlight's stroke SHALL miss no frame there; saving a highlight SHALL write the state within 10 ms;
opening the "Highlights" tab with 200 highlights SHALL take at most 50 ms; a page with highlights SHALL draw nothing
while idle and turn without more missed frames than a page without.

#### Scenario: Idle with highlights
- **WHEN** a page with three highlights is shown and nothing moves
- **THEN** no frames are drawn

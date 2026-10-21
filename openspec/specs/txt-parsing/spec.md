# txt-parsing Specification

## Purpose
TBD - created by archiving change markdown-txt. Update Purpose after archive.

## Requirements

### Requirement: A text file
A file SHALL be read as text when it is not a zip archive and not FB2, is not empty, has no zero byte in its first
4096 bytes (unless it starts with a UTF-16 byte order mark), and does not start — after a UTF-8 byte order mark and
whitespace, in any case — with `%PDF`, `<!DOCTYPE html` or `<html`. The decision MUST NOT depend on the file's name.
The encoding SHALL be decided as in the Android app: a UTF-8 or UTF-16 byte order mark decides; else valid UTF-8 (no
overlong forms, no surrogates) is UTF-8; else windows-1251. Line ends `\r\n` and `\r` SHALL be read as `\n`. A text
file larger than 16 MB SHALL be refused as "the file is damaged"; one with only whitespace, or that makes no block, as
"the book has no text". A text file without the marks of `markdown-parsing` SHALL be read as plain text.

#### Scenario: A book from lib.ru
- **WHEN** the reader opens a `.txt` in windows-1251
- **THEN** the book opens and its Russian text reads without mojibake

#### Scenario: Text without an extension
- **WHEN** a UTF-8 text file is named `notes`, without an extension
- **THEN** it opens as plain text, titled "notes"

#### Scenario: A binary file
- **WHEN** the reader opens a `.mobi` or a `.doc`
- **THEN** the message says "Couldn't open “name”: unsupported format — FB2, FB3, EPUB, TXT or Markdown needed"

#### Scenario: An empty file
- **WHEN** a `.txt` holds only spaces and line breaks
- **THEN** the message ends with "the book has no text"

### Requirement: Paragraphs of plain text
Plain text SHALL be split into paragraphs by one rule chosen over the whole file, counting the lines with text that are
neither chapter headings nor scene breaks (tabs read as spaces):
- when at least a third of them start with a space, but not all — a paragraph starts at a line with a space at its
  start, and lines without one continue it;
- else when they come in more than one group between blank lines and there are more lines than groups — a paragraph
  runs to a blank line;
- else every line with text is a paragraph.

Continuation lines SHALL be joined with a space; a word broken with `-` at the end of a line (a letter before the
hyphen) followed by a line starting in lower case SHALL be joined without the hyphen. Runs of spaces in a line SHALL be
one space. Blank lines MUST NOT make empty paragraphs. A line of only `*`, `-`, `_`, `~` and spaces, three of the marks
or more, SHALL be a scene break (an empty line). The typography SHALL be normalized as in FB2.

#### Scenario: Indents and hard line ends
- **WHEN** paragraphs start with three spaces and lines are cut at 72 characters
- **THEN** each paragraph is one paragraph of the book, justified, unbroken in the middle

#### Scenario: Blank lines between paragraphs
- **WHEN** paragraphs are separated by blank lines
- **THEN** the book has the same paragraphs as the file

#### Scenario: A line per paragraph
- **WHEN** the file has neither indents nor blank lines and every line is a whole paragraph
- **THEN** every line is a paragraph

### Requirement: Chapters of plain text
A line SHALL be a chapter heading when it starts the file or follows a blank line, is at most 80 characters (UTF-16
units, as on Android), and, in lower case, either starts with "глава", "часть", "пролог", "эпилог", "chapter", "part",
"prologue" or "epilogue" followed by nothing or by a space, `.`, `:`, `,`, `-`, `–` or `—` and anything, or is only up
to three digits or up to eight of the letters `ivxlcdm`, with an optional dot. Headings SHALL start chapters (a first
level heading block, the chapter's title) and make the contents; text before the first heading SHALL be an untitled
first chapter. Without headings the book SHALL be one chapter with empty contents. The book's title SHALL be the
file's name without the extension; it has no authors; its language for hyphenation SHALL be guessed from the text.

#### Scenario: A novel in TXT
- **WHEN** the file has the lines "ГЛАВА ПЕРВАЯ" and "ГЛАВА ВТОРАЯ", each after a blank line
- **THEN** the contents list two chapters, each starting a new page

#### Scenario: A note without chapters
- **WHEN** the reader opens `Список покупок.txt`
- **THEN** the library shows "Список покупок", one chapter, empty contents

### Requirement: The same blocks as on Android
For every text file the book SHALL have the same chapters, blocks and block texts as the Android app makes of it
(`:core:format-text`), so that a synced reading place lands on the same words. The Android app's tests of its text
reader SHALL be ported as tests of the core.

#### Scenario: A place from the phone
- **WHEN** the phone synced a place in the third paragraph of chapter 2 of a TXT book, and the same file is opened on
  the Mac
- **THEN** the Mac opens on the same words

## ADDED Requirements

### Requirement: Telling Markdown
A text file (`txt-parsing`) SHALL be read as Markdown when one of its lines is a heading (`#` to `######`, a space and
text), a code fence (three or more `` ` `` or `~` indented by at most three spaces, a backtick fence with no backtick
after it) or a table's header (a line with `|`) over its delimiter row (`|---|:-:|`, with `|` and `-`); or when at
least two of these are there: a link `[text](address)`, bold `**text**` or `__text__`, three or more list lines (`- `,
`* `, `+ `, `1. `, `1) ` with text, not a rule), a quote line (`>` with text). Else it SHALL be plain text. The
decision MUST NOT depend on the file's name.

#### Scenario: An answer from a language model
- **WHEN** the reader opens `answer.md` with `##` headings, lists and a code block
- **THEN** it opens as Markdown

#### Scenario: Markdown named .txt
- **WHEN** a file named `answer.txt` holds `#` headings and a table
- **THEN** it is read as Markdown

#### Scenario: A novel with stars between scenes
- **WHEN** a TXT novel has `* * *` lines between scenes and no other marks
- **THEN** it is read as plain text, the stars a scene break

### Requirement: Markdown metadata
A file starting with a block between `---` lines (front matter; closed within 200 lines, `...` closes too) that has at
least one `key: value` field MUST NOT show the block; the title SHALL come from `title`, the authors from `authors` or
`author` (a value or `- value` lines), the language from `lang` or `language`; quotes around values are dropped.
Without a `title`, when the first block is the only first-level heading of the file, it SHALL be the book's title and
not a block; else the title SHALL be the file's name without the extension. Without a language it SHALL be guessed
from the text.

#### Scenario: A title from the heading
- **WHEN** the file starts with `# Как устроен LineBreaker` and has no other first-level heading
- **THEN** the library shows "Как устроен LineBreaker"

#### Scenario: Front matter
- **WHEN** the file starts with `---`, `title: Заметки`, `author: Денис`, `---`
- **THEN** the library shows "Заметки" by "Денис", and the front matter's lines are not in the text

### Requirement: Markdown chapters and contents
Chapters SHALL start at headings of the top level among the headings left after the title; deeper headings SHALL stay
inside their chapter one level below per level. Text before the first chapter SHALL be an untitled first chapter. Every
heading with text SHALL be in the contents with its level. Without headings the book SHALL be one chapter. Setext
headings (`===` or `---` under a paragraph) SHALL be first and second level.

#### Scenario: A long answer in sections
- **WHEN** the file is `# Тема`, then `## Введение`, `## Детали`, `### Пример`, `## Итог`
- **THEN** the book is titled "Тема"; it has three chapters "Введение", "Детали", "Итог"; "Пример" is a heading inside
  "Детали" and a nested entry of the contents

### Requirement: Markdown blocks
The blocks SHALL be:
- a paragraph: its lines joined with spaces; two spaces or `\` at the end of a line, and `<br>`, a line break;
- a quote (`>`): its paragraphs quoted, nested quotes too;
- a code block (a fence, or lines indented by four spaces outside a list): preformatted, spaces and line breaks kept,
  no marks read inside;
- a list item: a list item paragraph starting with "• " for a bullet, the number as written and ". " for a numbered
  item, "☐ " or "☑ " for `[ ]` and `[x]`; nested items the same, one level deeper;
- a rule (`---`, `***`, `___`): a scene break (an empty line);
- a GitHub table (`|` rows under a delimiter row): a table of the book, its first row the header, alignment ignored;
  `\|` a pipe in a cell;
- an image: its caption in italics when it has one, else nothing;
- HTML tags and comments outside code MUST NOT be shown; `<br>` is a line break, other tags leave their text.

#### Scenario: Code in an answer
- **WHEN** the file has a ```` ```kotlin ```` block with lines indented by four spaces
- **THEN** the code is shown in the monospace face with the same indentation and line breaks, without hyphenation or
  justification

#### Scenario: A comparison table
- **WHEN** the file has a GitHub table of three columns and five rows
- **THEN** it is shown as a table, as in EPUB, and opens in the table view

### Requirement: Inline Markdown
`*text*` and `_text_` SHALL be italics, `**text**` and `__text__` bold, `~~text~~` struck out, `` `code` `` code, with
CommonMark's flanking rules (no italics inside `snake_case`). A link to `http`, `https` or `mailto` — `[text](address)`,
`<address>` or a bare `https://…` — SHALL open outside, as EPUB links do. A link `#anchor` SHALL lead to the heading
whose GitHub anchor (lower case, no punctuation, spaces as hyphens, `-1`, `-2` for repeats) it names, percent-encoded or
not; without such a heading, and for other addresses, only the link's text SHALL remain. A backslash before a mark
SHALL show the mark. The typography outside code SHALL be normalized as in FB2.

#### Scenario: Contents at the start of an answer
- **WHEN** the file starts with a list of links `[Детали](#детали)` and has `## Детали` below
- **THEN** a click on the link goes to the chapter "Детали", and "Back" returns

#### Scenario: A star in the text
- **WHEN** a paragraph says `2 \* 3 = 6`
- **THEN** it shows "2 * 3 = 6", not in italics

### Requirement: The same Markdown as on Android
For every Markdown file the book SHALL have the same chapters, blocks, block texts, contents and links as the Android
app makes of it (`:core:format-text`); the Android app's Markdown tests SHALL be ported as tests of the core.

#### Scenario: A synced place in an answer
- **WHEN** the phone synced a place in a list item of chapter 3 of `answer.md`, and the same file is opened on Linux
- **THEN** the book opens on the same words

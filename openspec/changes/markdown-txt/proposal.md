## Why

Readers ask whether Forzac opens Markdown: long answers from language models, notes and documentation read better
with book typesetting than in a chat window. Plain `.txt` is the same case and also the old format of Russian online
libraries (lib.ru and the like). Today both are refused: "unsupported format". Forzac for Android and the web reader
get Markdown and TXT in their change `markdown-txt`; the desktop follows with the same reading, so that a book opens
the same way on every device and a synced reading place (chapter, block, character) points at the same words.

No setting is added.

## What Changes

- Forzac opens Markdown (`.md`, `.markdown`) and plain text (`.txt`) from `⌘O`/`Ctrl+O`, a drop on the library,
  Finder, the file manager, `xdg-open` and the command line.
- The format is still decided by the content: a text file is one that is not a zip, not FB2, not PDF or HTML, without
  zero bytes, in UTF-8, UTF-16 or windows-1251; Markdown is told from plain text by the marks in the file.
- Markdown: headings make chapters and the contents; paragraphs, italics, bold, strikethrough, code, quotes, lists,
  tables and links go into the same book model as FB2 and EPUB. Images are not shown (a single file has none); their
  caption is.
- Plain text: paragraphs by blank lines, by indents or a line each; lines such as "Глава 1", "Часть вторая",
  "Chapter I" make chapters.
- A Markdown book's title comes from its first heading or its front matter's `title`, else from the file's name; plain
  text is titled by the file's name. A book of any format without a title is now titled by its file's name without the
  extension (it was the whole file name in the library, "Forzac" in the window).
- List items are set as the Android app sets them: no first-line indent, ragged right, nested items further right,
  wrapped lines under the text after the marker.
- The Books folder shows `.txt`, `.md` and `.markdown` files; adding a folder does not gather them (every README and
  log of a home folder would come along).
- The unsupported-format message and the empty library name TXT and Markdown; macOS offers Forzac in "Open With" for
  text and Markdown (never taking them over), Linux declares `text/markdown`.

## Non-goals

- CommonMark in full: lazy continuations, reference links `[text][label]`, footnotes `[^1]`, HTML entities, formulas
  `$…$` stay text.
- Images inside Markdown, `data:` images included.
- Opening text without a file (a clipboard, a drag of selected text).
- Declaring Forzac for every `text/plain` file on Linux: a desktop without a default for it could start opening logs
  and configuration files in a book reader.
- Drawing a task item's `☐`/`☑` as a box, as Android does: the characters are shown (from the fallback font when the
  bundled ones lack them).

## Capabilities

### New Capabilities
- `txt-parsing`: telling a text file and its encoding, plain text into paragraphs and chapters.
- `markdown-parsing`: telling Markdown from plain text, Markdown into chapters, contents, styles, tables and links.

### Modified Capabilities
- `fb2-parsing`: the one entry point sends text that is not FB2 to the text readers.
- `library`: the messages and the empty library name TXT and Markdown.
- `reader-window`: the unsupported-format message names TXT and Markdown.
- `books-folder`: text books in the folder and in their copies' names; not gathered when a folder is added.
- `macos-app`: Finder offers Forzac for text and Markdown.
- `linux-integration`: the desktop entry declares `text/markdown`.
- `typesetting`: list items.

## Impact

- Core: `core/src/text.rs` (detection, encoding, plain text) and `core/src/markdown.rs`, a port of the Android
  app's `:core:format-text`; `lib.rs` dispatch; `model::Kind::ListItem`; the typesetter's list items; the Books
  folder's extensions; `open` titles an untitled book by its file.
- GTK: the file dialog's filter; the desktop entry. macOS: `Info.plist` document types.
- Strings in the catalogue (English and Russian); README and the Mac's first-open page.

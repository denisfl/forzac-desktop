## 1. Core

- [x] 1.1 `text.rs`: `is_text` (empty, zip, zero bytes but UTF-16, `%PDF`, HTML), `decode` (byte order mark, strict UTF-8, windows-1251), `parse` (16 MB → damaged, blank → no text, Markdown or plain), `lib.rs`: zip → FB2 → text → not a book
- [x] 1.2 The plain-text reader, a port of Android's `PlainTextReader`: the three paragraph rules, joined lines and hyphenated words, scene breaks, chapter headings, typography
- [x] 1.3 `markdown.rs`, a port of Android's `MarkdownReader` and `MarkdownInline`: telling Markdown, front matter, blocks (headings, setext, paragraphs and breaks, quotes, fences and indented code, lists with levels and tasks, rules, tables, images, HTML), inline styles with flanking, links and anchors, chapters and contents
- [x] 1.4 `Kind::ListItem(level)` in the model; the typesetter's list items (hanging indent by the typeset marker, levels, one spacing group); task boxes as `Stroke` shapes (`ForzacStroke::shape`), a done task's text in the secondary ink; Kindle export
- [x] 1.5 `open` and the Books folder title an untitled book by its file's name (`library::file_title`); the folder reads `.txt`, `.md`, `.markdown`, names a copy without an extension by its content, and does not gather text when a folder is added
- [x] 1.6 Tests: Android's `TextParserTest`, `PlainTextReaderTest` and `MarkdownReaderTest` ported; encodings, binary files, file-name titles, the Books folder, list item layout; old tests' fake binaries given real headers (a JPEG, a PNG)
- [x] 1.7 Parity with the Android reader: both readers dumped 694 `.md`/`.txt` files and a set of hard cases; every chapter, block, text, style and link matched (the differences were empty files and HTML, which Android refuses before its reader)

## 2. Shells and texts

- [x] 2.1 Strings (English, Russian): the unsupported-format message and the empty library name TXT and Markdown
- [x] 2.2 GTK: `txt`, `md`, `markdown` in the file dialog; the desktop entry declares `text/markdown`
- [x] 2.3 macOS: `Info.plist` document types `public.plain-text` and `net.daringfireball.markdown` (imported) at rank Alternate
- [x] 2.5 Task boxes drawn by both shells: Core Graphics (`drawTaskBox`), `gsk` paths (`task_box`); checked on the Mac in the light and dark themes
- [x] 2.4 README, the Mac's first-open page, `docs/android-parity.md` (F9)

## 3. Check

- [x] 3.1 `cargo test -p forzac-core`, `cargo clippy -p forzac-core --all-targets -- -D warnings`, `cargo fmt --all --check`
- [x] 3.2 The GTK shell builds and passes `cargo clippy --workspace --all-targets -- -D warnings` in `forzac-debian-clippy` (not run on Linux)
- [x] 3.3 macOS (`macos/build.sh`, a test state, `--png`/`--panel`/`--mouse`): a Markdown answer — title from its heading, contents, nested lists with hanging lines, task boxes, code, quotes, a table and its view, links; a click on an anchor goes to its chapter with "Back"; a windows-1251 TXT with indents, chapters, a scene break and a hyphenated word; a JPEG named `.md` gets the new message
- [ ] 3.4 With the maintainer on the Mac: `⌘O` and a drop of a `.md` and a `.txt`, Finder's "Open With" offering Forzac for both while the default app stays; a TXT book synced from the phone opens on the same place

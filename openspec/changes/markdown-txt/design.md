## Context

The Android app reads TXT and Markdown in `:core:format-text` (`TextParser`, `PlainTextReader`, `MarkdownReader`,
`MarkdownInline`), shared with the web reader through WebAssembly. Reading positions sync between devices as
(chapter, block, offset in code points) (`sync`, `docs/sync-format.md`); a place found by block number lands on the
same words only when both apps make the same blocks with the same text. The desktop core is Rust with no Kotlin to
share, so the readers are written again.

## Goals / Non-Goals

**Goals:**
- The same chapters, blocks, block texts and contents as the Android app, character for character, for every TXT
  and Markdown file; styles and links the same as far as the reader sees them.
- Everything in the core, tested on its own; shells only learn the new file types.

**Non-Goals:**
- A better Markdown reader than Android's: where the Android reader is simple (numbering as written, a heading in a
  quote making a chapter), the desktop is equally simple. Improvements go to both apps in one change.

## Decisions

- **A port, rule for rule, not a reimplementation from the specs.** The specs leave room (what a chapter word may be
  followed by, how a list is numbered, whether spaces collapse); two readers written from them differ in exactly
  those places, and every difference moves synced places. The Android reader's regular expressions are written out
  as code (no `regex` dependency), with Java's ASCII `\s` (`text::is_space`) and UTF-16 lengths where Android counts
  them (`text::utf16_len`). Checked against the Android reader's own JVM build: both dumped 694 `.md`/`.txt` files
  found on the maintainer's machine and a set of hard cases; every block, text, style and link matched (the only
  differences were inputs Android rejects before its parser). The Android tests are ported to Rust tests.
- **Core, Rust:** `text.rs` (`is_text`, `decode`, `parse`, the plain-text reader), `markdown.rs` (`looks_like`, the
  block reader, inline styles, chapters). `lib.rs`: zip → FB2 → text → not a book; text is tried only when FB2 says
  "not a book", so FB2 and archives are untouched. Nothing crosses the C ABI: the shells call `forzac_open` as before.
- **Encodings:** `encoding_rs` (already a dependency) for the byte order mark, UTF-16 and windows-1251; UTF-8 checked
  with `std::str::from_utf8`, which is as strict as Android's check (no overlong forms or surrogates).
- **List items are `Kind::ListItem(level)`.** Android added the kind with a nesting level for its list layout; the
  desktop model mirrors Android's. The typesetter gets a hanging indent (`Spec::hang`: lines after the first are
  narrower and start right of the marker) and shifts a level 1.5 em right, four levels at most, as Android does.
  Items share one spacing group, so a list has space around it but not between its items. Kindle export sets them as
  plain left-aligned paragraphs.
- **Titles from file names in the core.** `open(path)` and the Books folder scan title an untitled book by its file's
  name without the book's extension (`library::file_title`), so the library, the window title, the running head and
  the cover all show "ответ", not "ответ.md" or "Forzac". This applies to an untitled FB2 too.
- **Books folder:** `.txt`, `.md`, `.markdown` join the extensions the folder is read by; a copy of a file without an
  extension gets `.md` or `.txt` by its content. Adding a folder skips text extensions: a home folder holds hundreds of
  READMEs, logs and licences. Opening one such file still works.
- **Shells:** GTK's file dialog filter gets `txt`, `md`, `markdown`; the desktop entry declares `text/markdown` but not
  `text/plain` (desktops pick a default handler from the declared types when none is set). macOS declares
  `public.plain-text` and `net.daringfireball.markdown` (imported, `.md`/`.markdown`) at rank Alternate: "Open With"
  offers Forzac, the default stays the editor's. The Mac's open panel and drops already accept any file.

## Risks / Trade-offs

- [The Android reader changes after this port] → its rules are pinned by its tests and by these ported ones; a change
  to either reader that moves blocks must land in both, and the dump comparison (Kotlin `Dump.java` against a Rust
  example) is the check — kept in the hand-over notes, not in the repository.
- [A TXT with a few Markdown marks is read as Markdown and its lines joined] → the same thresholds as Android; the
  worst case is still readable.
- [Characters `☐` `☑` missing from the bundled fonts] → glyph fallback (`typesetting`) finds them in the system's fonts.
- [A file in KOI8-R or CP866 reads as windows-1251 mojibake] → rare; same as Android.

## Migration Plan

None: such files were refused before. Rolling back removes the text branch; text books already in the Books folder
then fail as "unsupported format" and are hidden from the library, their files untouched.

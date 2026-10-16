## Context

The spike parser (`core/src/fb2.rs`) reads the main body: chapters, parts, styles, verse. It drops notes, images,
links and tables, which the typesetter will need next (`typesetting-complete`). The Android app's `Fb2Reader` is the
reference for behaviour; this change ports behaviour, not code.

## Goals / Non-Goals

**Goals:** the model carries notes, links, images, tables and contents; the parser fills them with Android's rules;
damaged files degrade the same way; corpus tests.

**Non-Goals:** drawing any of it (`typesetting-complete`), FB3 (`epub`), MathML.

## Decisions

- **Core only.** Everything here is in `core/`; shells are untouched except that the typesetter skips image and table
  blocks until `typesetting-complete` draws them. The C ABI does not change.
- **Model additions**: `Book.notes: HashMap<String, Note>`, `Book.links: HashMap<String, LinkTarget>`,
  `Book.toc: Vec<TocEntry>`, `Book.images: HashMap<String, Image>`, `Book.cover`, `Book.annotation`;
  `Block::Image { id }`, `Block::TableRow { table: Arc<Table>, row }`; `Style::Link`; a `target` on style ranges for
  note ids and link keys. `Arc` so a book can be moved to a layout thread later.
- **Images in memory.** Books are a few megabytes; decoded bytes stay in the book (Android writes them to files
  because of its process limits). Base64 decoding with the `base64` crate.
- **Link targets without object identity.** Android finds where a block ended up by object identity. Here every block
  gets a sequence number when it is read; ids map to the first and last sequence numbers of their element or section;
  after chapters are laid out, sequence numbers map to Locators.
- **Errors keep partial results**: the reader already returns what was read past the root element; the open section is
  emitted before the error is returned.

## Risks / Trade-offs

- Memory: a book with large images keeps them in memory twice briefly (base64 text and bytes) → decode per binary
  and drop the text at once.
- Corpus coverage is limited to the books at hand (six in `~/Downloads`) → the corpus test is the place to add more.

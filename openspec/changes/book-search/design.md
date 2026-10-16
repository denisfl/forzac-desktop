## Context

Android searches with `Book.search` (`core/model/.../Search.kt`): normalised characters, matches in paragraphs and
headings (notes are not searched), a snippet of 40 characters before and 80 after cut at words. Its search screen is a
field, a count and a list of snippets with the section and page.

## Decisions

**Rules ported, PDF parts left out.** The desktop has no PDF: the end-of-line hyphen rules go. Table rows are left out
for now (a row's text is several cells); paragraphs and headings are searched. Offsets are byte offsets, as the
desktop `Locator`.

**Results at once, off the main thread.** A whole book is searched in one pass (Rust over the model: milliseconds on a
Raspberry Pi 5 for a long book), so results arrive together instead of streaming. Each search runs on a worker thread
and a newer query makes an older result stale.

**Highlight from the layout.** Lines keep, for each glyph, the text offset it was shaped from, its x and advance. The
chapter layout answers "rectangles of text `[from, to)` on page `i`" and "… between column heights `y0` and `y1`",
walking the same lines its renderers draw. Shells draw them as rounded rectangles in the ink at 14 % (Android's
selection colour) over the page, like the `Tab` focus.

**Panel.** A side panel like the contents: a search icon and the field, a clear button, a rule, "N matches" (or "No
matches"), the list (snippet in the book serif 16/23 at 78 % ink with the match in full ink and bold; "section · N%" in
13 px secondary). `Enter` in the field goes to the selected match (the first), `↓` moves into the list.

## Risks / Trade-offs

- Glyph offsets add about 12 bytes per glyph to a kept chapter layout (a long chapter: under a megabyte).

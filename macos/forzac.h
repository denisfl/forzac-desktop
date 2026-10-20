// C ABI of forzac-core (core/src/ffi.rs) for the macOS shell.
#pragma once
#include <stdbool.h>
#include <stdint.h>
#include <stddef.h>

typedef struct ForzacDocument ForzacDocument;
typedef struct ForzacState ForzacState;

typedef struct {
    uint16_t font;          // 0-3 the bundled PT Serif faces; later ids other families and fallback fonts
    uint8_t ink;            // 0 text, 1 link, 2 secondary
    float size;             // logical pixels
    uint32_t count;
    const uint16_t *glyphs;
    const float *positions; // count x/y pairs: glyph origins on the baseline, y down
} ForzacRun;

typedef struct {
    float x, y, w, h;       // y down
} ForzacRect;

typedef struct {
    const uint8_t *bytes;   // the image file, owned by the document; the same pointer for the same image
    size_t len;
    ForzacRect rect;
} ForzacImage;

typedef struct {
    ForzacRect rect;
    uint8_t strength;       // 1 light, 2 medium, 3 heavy
} ForzacRule;

typedef struct {
    ForzacRect rect;
    uint8_t ink;            // 0 text, 1 link, 2 secondary
} ForzacStroke;             // a line through struck-out text

typedef struct {
    uint32_t chapter;
    uint32_t block;
    uint32_t offset;
} ForzacLocator;

typedef struct {
    ForzacRect rect;
    uint8_t kind;           // 0 note marker, 1 link, 2 image (opens in the view), 3 table (opens in the view), 4 a
                            // highlight (the shell's own; the core never sets it)
    const char *target;     // note id, link key or image id; empty for a table
    ForzacLocator at;       // a table's first row
} ForzacArea;

typedef struct {
    const ForzacRun *runs;
    uint32_t run_count;
    const ForzacImage *images;
    uint32_t image_count;
    const ForzacRule *rules;
    uint32_t rule_count;
    const ForzacArea *areas;
    uint32_t area_count;
    const ForzacStroke *strokes;
    uint32_t stroke_count;
} ForzacPage;

// Indices into the presets; theme -1 follows the system's light or dark.
typedef struct {
    int32_t theme;          // Light, Sepia, Dark, Dawn, Wave, Forest
    uint32_t size;          // below forzac_settings_size_count()
    uint32_t family;        // PT Serif, Literata, Source Serif, PT Sans
    uint32_t spacing;       // compact, normal, loose
    uint32_t align;         // justified, left
    uint32_t margins;       // narrow, medium, wide
    uint32_t turning;       // slide, scroll
} ForzacSettings;

typedef struct {
    const char *title;
    uint32_t level;         // 1 top level
    ForzacLocator at;
    float position;         // share of the book before it, 0-1
} ForzacTocEntry;

typedef struct {
    const char *path;
    const char *title;
    const char *authors;
    float progress;
    const char *id;
    bool opened;            // false: in the Books folder, never opened
    const char *series;     // empty: not in a series
    float number;           // in the series; negative: none
    const char *shelves;    // the reader's shelves it is on, one per line
} ForzacRecent;

// A generated cover: cloth or marbled paper (forzac_marble with `seed`) under `cloth`, a spine, the title's size
// (a share of 11.5 % of the cover's width) and its most lines.
typedef struct {
    bool marble;
    uint32_t cloth;
    uint32_t spine;
    int32_t seed;
    float title_scale;
    uint32_t title_lines;
} ForzacCoverStyle;

// Finds a font file with character `ch`: writes its path into `path` and the face index into `*index`.
typedef bool (*ForzacFallback)(uint32_t ch, char *path, size_t capacity, uint32_t *index);

// NULL when the file is not a readable book; *error (if not NULL): 1 not a book, 2 no text, 3 DRM-protected,
// 4 a damaged archive, 5 the file cannot be read.
ForzacDocument *forzac_open(const char *path, uint32_t *error);
const char *forzac_title(const ForzacDocument *doc);
// The book's identity: the key of its cached cover.
const char *forzac_book_id(const ForzacDocument *doc);
const char *forzac_authors(const ForzacDocument *doc);
uint32_t forzac_chapter_count(const ForzacDocument *doc);
// Typesets `chapter` for the window with `settings`, makes it current; returns its page count.
uint32_t forzac_paginate(ForzacDocument *doc, uint32_t chapter, float width, float height, const ForzacSettings *settings);
// Pages of the current chapter. A ForzacPage is valid until the next call on the same document.
ForzacPage forzac_page(ForzacDocument *doc, uint32_t page);
ForzacLocator forzac_page_start(ForzacDocument *doc, uint32_t page);
// The next page's start; after the chapter's last page the next chapter's start.
ForzacLocator forzac_page_end(ForzacDocument *doc, uint32_t page);
uint32_t forzac_page_of(ForzacDocument *doc, ForzacLocator at);
// The current style's top margin and line height.
void forzac_metrics(const ForzacDocument *doc, float *margin_top, float *line_height);
// The bookmark ribbon's centre and the left edge of the corner (in the top margin) where a click toggles a bookmark.
void forzac_ribbon(const ForzacDocument *doc, float *centre, float *corner);
// "Scroll": a chapter as one continuous column (y from the top of the text).
float forzac_flow_height(ForzacDocument *doc, uint32_t chapter);
ForzacPage forzac_flow(ForzacDocument *doc, uint32_t chapter, float y0, float y1);
float forzac_flow_y(ForzacDocument *doc, ForzacLocator at);
ForzacLocator forzac_flow_locator(ForzacDocument *doc, uint32_t chapter, float y);
// Contents, valid until the document is closed.
const ForzacTocEntry *forzac_contents(ForzacDocument *doc, uint32_t *count);
float forzac_progress(const ForzacDocument *doc, ForzacLocator at);
const uint8_t *forzac_cover(const ForzacDocument *doc, size_t *len);
// A note for a panel `width` wide into *out; returns its height, negative when there is no such note.
float forzac_note(ForzacDocument *doc, const char *id, float width, const ForzacSettings *settings, ForzacPage *out);
// Strings below are valid until the next string from the document.
const char *forzac_note_title(ForzacDocument *doc, const char *id);
// 1 a place (*at), 2 a note (*text its id), 3 outside the book (*text the address), 4 a short place for the note
// sheet (forzac_link_preview; *at its start); 0 unknown.
uint8_t forzac_link(ForzacDocument *doc, const char *key, ForzacLocator *at, const char **text);
// A link's short target for a panel `width` wide into *out; returns its height, negative when it has none.
float forzac_link_preview(ForzacDocument *doc, const char *key, float width, const ForzacSettings *settings, ForzacPage *out);
// The end page after the book's last page, for the current style.
ForzacPage forzac_end_page(ForzacDocument *doc);
// An image's file bytes (the target of a kind 2 area; the same pointer pages give); NULL when there is no such image.
const uint8_t *forzac_image(const ForzacDocument *doc, const char *id, size_t *len);
// A table typeset for the view (the `at` of a kind 3 area) at `zoom` times the text size; NULL when `at` is not a
// table's first row. Its page is drawn from the origin, *width × *height, and stays valid until the table is freed.
typedef struct ForzacTable ForzacTable;
ForzacTable *forzac_table(const ForzacDocument *doc, ForzacLocator at, float zoom, const ForzacSettings *settings);
ForzacPage forzac_table_page(const ForzacTable *table, float *width, float *height);
void forzac_table_free(ForzacTable *table);
// Rectangles over the text [from, to) (a search match) on a page of the current chapter, or between heights of a
// chapter's column; up to `capacity` into `out`, returns how many.
uint32_t forzac_marks(ForzacDocument *doc, uint32_t page, ForzacLocator from, ForzacLocator to, ForzacRect *out, uint32_t capacity);
uint32_t forzac_flow_marks(ForzacDocument *doc, uint32_t chapter, float y0, float y1, ForzacLocator from, ForzacLocator to,
                           ForzacRect *out, uint32_t capacity);
void forzac_close(ForzacDocument *doc);

// Bookmarks, held by the document (load and save them with the state).
typedef struct {
    ForzacLocator at;
    const char *section;    // NULL when there is none
    const char *snippet;    // the text from it
    float position;         // share of the book before it, 0-1
} ForzacBookmark;
// Sets one at `from` when none lies in [from, to), else removes those that do; true when it set one.
bool forzac_bookmark_toggle(ForzacDocument *doc, ForzacLocator from, ForzacLocator to);
void forzac_bookmark_remove(ForzacDocument *doc, ForzacLocator at);
bool forzac_bookmarked(const ForzacDocument *doc, ForzacLocator from, ForzacLocator to);
// In reading order; each valid until the next call of the same function.
const ForzacLocator *forzac_bookmark_places(ForzacDocument *doc, uint32_t *count);
const ForzacBookmark *forzac_bookmarks(ForzacDocument *doc, uint32_t *count);

// Words and highlights.
typedef struct {
    ForzacLocator start;    // the whole word's text [start, end): a hyphenated word's pieces share it
    ForzacLocator end;
    ForzacRect rect;        // this piece on its line
} ForzacWord;
typedef struct {
    ForzacLocator start;
    ForzacLocator end;
} ForzacRange;
typedef struct {
    ForzacLocator start;
    ForzacLocator end;
    const char *text;
    const char *section;    // NULL when there is none
    float position;         // share of the book before it, 0-1
} ForzacQuote;
// The words drawn on a page of the current chapter, or between heights of a chapter's column (forzac_flow's
// coordinates); in reading order, valid until the next words from the document.
const ForzacWord *forzac_page_words(ForzacDocument *doc, uint32_t page, uint32_t *count);
const ForzacWord *forzac_flow_words(ForzacDocument *doc, uint32_t chapter, float y0, float y1, uint32_t *count);
// Over a word array: the word under a point (else the nearest), the next/previous word, the word on the next/previous
// line, the word starting at (or after) a place; -1 for none.
int32_t forzac_word_at(const ForzacWord *words, uint32_t count, float x, float y);
int32_t forzac_word_step(const ForzacWord *words, uint32_t count, uint32_t i, int32_t delta);
int32_t forzac_word_line(const ForzacWord *words, uint32_t count, uint32_t i, int32_t delta);
int32_t forzac_word_find(const ForzacWord *words, uint32_t count, ForzacLocator at);
ForzacRange forzac_block_range(const ForzacDocument *doc, ForzacLocator at);
// A range's text as copied, and signed for sharing; valid until the next string from the document.
const char *forzac_text(ForzacDocument *doc, ForzacLocator from, ForzacLocator to);
const char *forzac_share_text(ForzacDocument *doc, ForzacLocator from, ForzacLocator to);
// Highlights, held by the document (load and save them with the state).
void forzac_quote_add(ForzacDocument *doc, ForzacLocator from, ForzacLocator to);
void forzac_quote_remove(ForzacDocument *doc, ForzacLocator from, ForzacLocator to);
// In reading order; each valid until the next call of the same function.
const ForzacRange *forzac_quote_places(ForzacDocument *doc, uint32_t *count);
const ForzacQuote *forzac_quotes(ForzacDocument *doc, uint32_t *count);
// All highlights in Android's export format; valid until the next string from the document.
const char *forzac_quotes_export(ForzacDocument *doc);

// Search: a handle to the book for another thread, then the matches (owned until freed).
typedef struct ForzacBook ForzacBook;
typedef struct ForzacSearch ForzacSearch;
typedef struct {
    ForzacLocator start;
    ForzacLocator end;
    const char *snippet;
    uint32_t match_start;   // bytes of the snippet
    uint32_t match_end;
    const char *section;    // NULL when there is none
    float position;         // share of the book before it, 0-1
} ForzacHit;
ForzacBook *forzac_book(const ForzacDocument *doc);
void forzac_book_release(ForzacBook *book);
ForzacSearch *forzac_search(const ForzacBook *book, const char *query);
const ForzacHit *forzac_search_hits(const ForzacSearch *search, uint32_t *count);
void forzac_search_free(ForzacSearch *search);

// The book's pages (the menu, the foot, the lists): a count made for the current window and settings on the main
// thread, run on a background one, installed on the main thread (true: the pages' feet changed, draw them again). Until
// installed, and after a style change, forzac_book_pages is 0. forzac_page draws "N of M" in the foot once installed.
typedef struct ForzacCount ForzacCount;
ForzacCount *forzac_count_new(const ForzacDocument *doc, float width, float height, const ForzacSettings *settings);
void forzac_count_run(const ForzacCount *count);   // any thread; blocks while typesetting the book
void forzac_count_stop(const ForzacCount *count);  // any thread
void forzac_count_moved(const ForzacCount *count); // any thread: the reader moved, the count waits for a quiet 0.5 s
bool forzac_count_install(ForzacDocument *doc, const ForzacCount *count);
void forzac_count_free(ForzacCount *count);
uint32_t forzac_book_pages(const ForzacDocument *doc);
uint32_t forzac_book_page_of(const ForzacDocument *doc, ForzacLocator at);   // from 0
ForzacLocator forzac_book_page_start(const ForzacDocument *doc, uint32_t page);
// The book page of each chapter's first page; valid until the next call.
const uint32_t *forzac_chapter_starts(ForzacDocument *doc, uint32_t *count);
// The section `at` is in, else the book's title (valid until the next string); minutes left in its chapter.
const char *forzac_section(ForzacDocument *doc, ForzacLocator at);
uint32_t forzac_minutes_left(const ForzacDocument *doc, ForzacLocator at);

// The interface language from the preferred languages, comma separated ("ru-RU,en"); then the catalogue: the Russian
// of an English string (NULL: keep the English), a count with its noun ("{n} match", "{n} matches"; valid until the
// next call), the help (platform 0 Linux, 1 macOS: the title, then "# section" and "keys\tmeaning" lines, the support line
// last; keys separated by spaces).
void forzac_set_language(const char *locales);
const char *forzac_tr(const char *english);
const char *forzac_tr_count(uint32_t n, const char *one, const char *other);
const char *forzac_help(uint32_t platform);

void forzac_set_assets(const char *dir);
void forzac_set_fallback(ForzacFallback provider);
// The bundled PT Serif face of `font`; NULL for a font read from a file.
const uint8_t *forzac_font_data(uint16_t font, size_t *len);
// The file of a font read from a file; false for a bundled face.
bool forzac_font_file(uint16_t font, char *path, size_t capacity, uint32_t *index);
// The variation-axis values the font was shaped at (tags big-endian four letters); returns how many.
uint32_t forzac_font_variations(uint16_t font, uint32_t *tags, float *values, uint32_t capacity);
bool forzac_font_is_color(uint16_t font);

ForzacSettings forzac_settings_default(void);
uint32_t forzac_settings_theme(const ForzacSettings *settings, bool system_dark);
float forzac_settings_text_size(const ForzacSettings *settings);
// Whether a window this size shows two pages side by side (forzac_paginate then makes pages half its width).
bool forzac_settings_spread(const ForzacSettings *settings, float width, float height);
// How far a spread's pages are drawn towards the gutter: the left page right, the right page left; 0 if not a spread.
float forzac_settings_spread_inset(const ForzacSettings *settings, float width, float height);
uint32_t forzac_settings_size_count(void);
// setting: 0 theme, 1 font, 2 size, 3 spacing, 4 alignment, 5 margins, 6 pages.
void forzac_settings_step(ForzacSettings *settings, uint32_t setting, int32_t delta, bool system_dark);

// Reading state: positions, the library's books and settings in the file at `path`.
ForzacState *forzac_state_open(const char *path);
bool forzac_state_position(const ForzacState *state, const ForzacDocument *doc, ForzacLocator *out);
// `finished`: the end page was shown; the book reads 100 %.
bool forzac_state_save_position(ForzacState *state, const ForzacDocument *doc, ForzacLocator at, bool finished);
// The last opened first, files that exist only; valid until the next call on `state`.
const ForzacRecent *forzac_state_recent(ForzacState *state, uint32_t n, uint32_t *count);
// Removes the book from the library (its bookmarks and highlights stay).
bool forzac_state_forget(ForzacState *state, const char *id);
// Reads the file again after another handle wrote it.
void forzac_state_reload(ForzacState *state);

// The Books folder (books-folder). forzac_books_folder makes it in `home` when missing and, the first time, copies the
// remembered books in; forzac_import copies a book in and returns the path to open (null on failure, `error` as
// forzac_open's); forzac_library_books reads the folder: the remembered books, the last opened first, then the
// folder's books never opened. All three may take a while: call them on a background queue with a state handle of
// its own, then forzac_state_reload the main one. Strings and the list are valid until the next call on `state`.
const char *forzac_books_folder(ForzacState *state, const char *home);
const char *forzac_import(ForzacState *state, const char *path, uint32_t *error);
const ForzacRecent *forzac_library_books(ForzacState *state, uint32_t *count);

// Adding a folder (folder-import): forzac_import_folder copies the books in `src` and its subfolders into the Books
// folder, calling `progress(ctx, done, found)` on the calling thread before each file and at the end; on a background
// queue with a state handle of its own, after forzac_books_folder. forzac_added_message is the message for the reader
// afterwards, null when every book found was added; valid until the next call on `state`.
typedef struct {
    uint32_t added;
    uint32_t known;
    uint32_t failed;
} ForzacAdded;
ForzacAdded forzac_import_folder(ForzacState *state, const char *src, void (*progress)(void *ctx, uint32_t done, uint32_t found), void *ctx);
const char *forzac_added_message(ForzacState *state, ForzacAdded added, const char *name);
// Changing the Books folder (books-folder-setting): makes `path` the folder, moving the books there when `move`; on a
// background queue with a state handle of its own. False when `path` is not a writable folder; `*message` is what to
// tell the reader (null: nothing), valid until the next call on `state`.
bool forzac_change_books_folder(ForzacState *state, const char *path, bool move, const char **message);

// The library: search rank (lower first, -1 no match), generated covers, the empty state's sample page (lines: head,
// title, subtitle, folio, paragraphs; valid for good).
int32_t forzac_library_rank(const char *query, const char *title, const char *authors, const char *series);

// The library's views (library-metadata): forzac_library_arrange groups `n` books for `view` (0 Recent, 1 Title,
// 2 Author, 3 Series, 4 Shelves); group i holds books[starts[i]..starts[i + 1]] under headings[i] (null: none). forzac_library_step
// moves the selection through `n` laid-out groups given as (books, columns) pairs. forzac_book_info reads "About the
// book" from an open document.
typedef struct {
    const char *title;
    const char *authors;
    const char *series;     // empty: none
    float number;           // negative: none
    float progress;
    bool opened;
    const char *shelves;    // one per line
} ForzacBookKey;
typedef struct ForzacGroupsHandle ForzacGroupsHandle;
typedef struct {
    uint32_t count;
    const uint32_t *starts;
    const char *const *headings;
    const uint32_t *books;
} ForzacGroups;
ForzacGroupsHandle *forzac_library_arrange(const ForzacBookKey *books, uint32_t n, uint32_t view);
const ForzacGroups *forzac_groups(const ForzacGroupsHandle *groups);
void forzac_groups_free(ForzacGroupsHandle *groups);
uint32_t forzac_library_step(const uint32_t *sizes, uint32_t n, uint32_t at, int32_t dx, int32_t dy);
uint32_t forzac_library_view(const ForzacState *state);
// The shelves (library-shelves): every shelf with a book on it, one per line (valid until the next call on `state`);
// putting a book on a shelf or taking it off.
const char *forzac_shelf_names(ForzacState *state);
bool forzac_set_shelf(ForzacState *state, const char *id, const char *name, bool on);
bool forzac_set_library_view(ForzacState *state, uint32_t view);
typedef struct ForzacBookInfoHandle ForzacBookInfoHandle;
typedef struct {
    const char *title;
    const char *authors;
    const char *series;
    const char *details;    // genres · year · language
    const char *annotation;
} ForzacBookInfo;
ForzacBookInfoHandle *forzac_book_info(const ForzacState *state, const ForzacDocument *doc);
const ForzacBookInfo *forzac_book_info_view(const ForzacBookInfoHandle *info);
void forzac_book_info_free(ForzacBookInfoHandle *info);

// Editing a book's details (metadata-editing): forzac_edit_fields fills the "Edit…" form for an open document with
// the details as the library shows them; forzac_save_edits keeps what the form changes (the file is not touched);
// forzac_reset_edits drops the book's edits.
typedef struct ForzacFieldsHandle ForzacFieldsHandle;
typedef struct {
    const char *title;
    const char *authors;    // comma-separated
    const char *series;
    const char *number;
    const char *genres;     // comma-separated
} ForzacFields;
ForzacFieldsHandle *forzac_edit_fields(const ForzacState *state, const ForzacDocument *doc);
const ForzacFields *forzac_fields_view(const ForzacFieldsHandle *fields);
void forzac_fields_free(ForzacFieldsHandle *fields);
bool forzac_save_edits(ForzacState *state, const ForzacDocument *doc, ForzacFields fields);
bool forzac_reset_edits(ForzacState *state, const char *id);
ForzacCoverStyle forzac_cover_style(const char *id, const char *title);
void forzac_marble(int32_t seed, uint32_t width, uint32_t height, uint8_t *out);
const char *forzac_sample_page(void);

// Whether a newer version is out (update-check): the shell fetches forzac_update_releases() when forzac_update_due,
// hands the answer to forzac_update_record; forzac_update_notice is the version to offer (null when none; valid until
// the next call on `state`), forzac_update_dismiss hides it until a newer one; forzac_update_page is where to download.
const char *forzac_update_releases(void);
const char *forzac_update_page(void);
bool forzac_update_due(const ForzacState *state);
bool forzac_update_record(ForzacState *state, const char *answer);
const char *forzac_update_notice(ForzacState *state);
bool forzac_update_dismiss(ForzacState *state);

void forzac_state_load_bookmarks(const ForzacState *state, ForzacDocument *doc);
bool forzac_state_save_bookmarks(ForzacState *state, const ForzacDocument *doc);
void forzac_state_load_quotes(const ForzacState *state, ForzacDocument *doc);
bool forzac_state_save_quotes(ForzacState *state, const ForzacDocument *doc);
ForzacSettings forzac_state_settings(const ForzacState *state);
bool forzac_state_save_settings(ForzacState *state, const ForzacSettings *settings);
void forzac_state_close(ForzacState *state);

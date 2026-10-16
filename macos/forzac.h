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
    ForzacRect rect;
    uint8_t kind;           // 0 note marker, 1 link
    const char *target;     // note id or link key
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

typedef struct {
    uint32_t chapter;
    uint32_t block;
    uint32_t offset;
} ForzacLocator;

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
} ForzacRecent;

// Finds a font file with character `ch`: writes its path into `path` and the face index into `*index`.
typedef bool (*ForzacFallback)(uint32_t ch, char *path, size_t capacity, uint32_t *index);

// NULL when the file is not a readable book; *error (if not NULL): 1 not a book, 2 no text, 3 DRM-protected,
// 4 a damaged archive, 5 the file cannot be read.
ForzacDocument *forzac_open(const char *path, uint32_t *error);
const char *forzac_title(const ForzacDocument *doc);
const char *forzac_authors(const ForzacDocument *doc);
uint32_t forzac_chapter_count(const ForzacDocument *doc);
// Typesets `chapter` for the window with `settings`, makes it current; returns its page count.
uint32_t forzac_paginate(ForzacDocument *doc, uint32_t chapter, float width, float height, const ForzacSettings *settings);
// Pages of the current chapter. A ForzacPage is valid until the next call on the same document.
ForzacPage forzac_page(ForzacDocument *doc, uint32_t page);
ForzacLocator forzac_page_start(ForzacDocument *doc, uint32_t page);
uint32_t forzac_page_of(ForzacDocument *doc, ForzacLocator at);
// The current style's top margin and line height.
void forzac_metrics(const ForzacDocument *doc, float *margin_top, float *line_height);
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
// Rectangles over the text [from, to) (a search match) on a page of the current chapter, or between heights of a
// chapter's column; up to `capacity` into `out`, returns how many.
uint32_t forzac_marks(ForzacDocument *doc, uint32_t page, ForzacLocator from, ForzacLocator to, ForzacRect *out, uint32_t capacity);
uint32_t forzac_flow_marks(ForzacDocument *doc, uint32_t chapter, float y0, float y1, ForzacLocator from, ForzacLocator to,
                           ForzacRect *out, uint32_t capacity);
void forzac_close(ForzacDocument *doc);

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
uint32_t forzac_settings_size_count(void);
// setting: 0 theme, 1 font, 2 size, 3 spacing, 4 alignment, 5 margins, 6 pages.
void forzac_settings_step(ForzacSettings *settings, uint32_t setting, int32_t delta, bool system_dark);

// Reading state: positions, recent books and settings in the file at `path`.
ForzacState *forzac_state_open(const char *path);
bool forzac_state_position(const ForzacState *state, const ForzacDocument *doc, ForzacLocator *out);
// `finished`: the end page was shown; the book reads 100 %.
bool forzac_state_save_position(ForzacState *state, const ForzacDocument *doc, ForzacLocator at, bool finished);
// Valid until the next call on `state`.
const ForzacRecent *forzac_state_recent(ForzacState *state, uint32_t n, uint32_t *count);
ForzacSettings forzac_state_settings(const ForzacState *state);
bool forzac_state_save_settings(ForzacState *state, const ForzacSettings *settings);
void forzac_state_close(ForzacState *state);

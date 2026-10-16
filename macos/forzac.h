// C ABI of forzac-core (core/src/ffi.rs) for the macOS shell.
#pragma once
#include <stdbool.h>
#include <stdint.h>
#include <stddef.h>

typedef struct ForzacDocument ForzacDocument;

typedef struct {
    float width;
    float height;
    float text_size;
} ForzacStyle;

typedef struct {
    uint16_t font;          // 0-3 the bundled faces (regular, italic, bold, bold italic), later ids fallback fonts
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
} ForzacPage;

typedef struct {
    uint32_t chapter;
    uint32_t block;
    uint32_t offset;
} ForzacLocator;

// Finds a font file with character `ch`: writes its path into `path` and the face index into `*index`.
typedef bool (*ForzacFallback)(uint32_t ch, char *path, size_t capacity, uint32_t *index);

ForzacDocument *forzac_open(const char *path);
const char *forzac_title(const ForzacDocument *doc);
uint32_t forzac_chapter_count(const ForzacDocument *doc);
uint32_t forzac_paginate(ForzacDocument *doc, uint32_t chapter, const ForzacStyle *style);
// Valid until the next call on the same document.
ForzacPage forzac_page(ForzacDocument *doc, uint32_t page);
ForzacLocator forzac_page_start(const ForzacDocument *doc, uint32_t page);
uint32_t forzac_page_of(const ForzacDocument *doc, ForzacLocator at);
// The bundled font file of `font`; NULL for a fallback font.
const uint8_t *forzac_font_data(uint16_t font, size_t *len);
// The file of fallback font `font`; false for a bundled face.
bool forzac_font_file(uint16_t font, char *path, size_t capacity, uint32_t *index);
bool forzac_font_is_color(uint16_t font);
void forzac_set_fallback(ForzacFallback provider);
void forzac_close(ForzacDocument *doc);

// Reading state: positions and settings in the file at `path`.
typedef struct ForzacState ForzacState;
ForzacState *forzac_state_open(const char *path);
bool forzac_state_position(const ForzacState *state, const ForzacDocument *doc, ForzacLocator *out);
bool forzac_state_save_position(ForzacState *state, const ForzacDocument *doc, ForzacLocator at);
// Valid until the next call on `state`; NULL when not set.
const char *forzac_state_setting(ForzacState *state, const char *key);
bool forzac_state_set_setting(ForzacState *state, const char *key, const char *value);
void forzac_state_close(ForzacState *state);

// C ABI of forzac-core (core/src/ffi.rs) for the macOS shell.
#pragma once
#include <stdint.h>
#include <stddef.h>

typedef struct ForzacDocument ForzacDocument;

typedef struct {
    float width;
    float height;
    float text_size;
} ForzacStyle;

typedef struct {
    uint8_t face;          // 0 regular, 1 italic, 2 bold, 3 bold italic
    float size;            // logical pixels
    uint32_t count;
    const uint16_t *glyphs;
    const float *positions; // count x/y pairs: glyph origins on the baseline, y down
} ForzacRun;

typedef struct {
    uint32_t chapter;
    uint32_t block;
    uint32_t offset;
} ForzacLocator;

ForzacDocument *forzac_open(const char *path);
const char *forzac_title(const ForzacDocument *doc);
uint32_t forzac_chapter_count(const ForzacDocument *doc);
uint32_t forzac_paginate(ForzacDocument *doc, uint32_t chapter, const ForzacStyle *style);
uint32_t forzac_page_runs(ForzacDocument *doc, uint32_t page, const ForzacRun **out);
ForzacLocator forzac_page_start(const ForzacDocument *doc, uint32_t page);
uint32_t forzac_page_of(const ForzacDocument *doc, ForzacLocator at);
const uint8_t *forzac_font_data(uint8_t face, size_t *len);
void forzac_close(ForzacDocument *doc);

## Why

The spike parser reads the main text of an FB2 book. It skips notes, images, links and tables and has not met the
broken files that are common in the wild. FB2 is the most frequent format among Russian-language readers, so it has
to be as robust as on Android.

## What Changes

- Notes and comments bodies become footnotes addressable from their markers.
- Images (`<binary>`, base64) and the cover; links inside the book and to the web.
- Tables, `subtitle`, `text-author`, `date`, `cite`, `epigraph`, `poem` in every position the format allows.
- Encodings from the XML declaration and BOM; entities used without a DTD.
- Broken and truncated files: whatever was read before the error is kept; a zip bomb limit for `.fb2.zip`.
- A corpus test over a local directory of real books (`FORZAC_CORPUS`), skipped when it is not set.

## Capabilities

### New Capabilities
- `fb2-parsing`: what is read from FB2 and `.fb2.zip`, how broken files behave.

### Modified Capabilities
<!-- None. -->

## Impact

`core/src/fb2.rs` and the model (`notes`, `links`, images). Ported from the Android app's `Fb2Reader`.

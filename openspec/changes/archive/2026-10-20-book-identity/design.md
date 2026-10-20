# Design

## Context

`state::book_id` hashes the length and the first and last 64 KiB with FNV-1a 64 (16 hex digits). The id keys the
state file's `book`, `bookmark`, `quote` and `file` lines, the shells' cover thumbnails and the generated cover's
style (`library::cover_style`, ported from Android's `Covers.kt`, which picks by Android's id). Android's id is the
first 16 bytes of SHA-256 over the whole file, hashed while the file is copied in (`BookImport.copy`); it stores the
file unchanged, as the desktop's Books folder does, so equal files give equal ids. Both shells call
`library::books_folder` on a background thread before every library scan and every import.

## Decisions

**1. SHA-256 from the `sha2` crate.** Hashing the whole file is the price of matching Android. `sha2` uses the CPU's
SHA instructions on Apple Silicon and the Pi 5's Cortex-A76 (ARMv8 crypto extensions) through `cpufeatures`: well over
1 GB/s, so a 50 MB book adds tens of milliseconds to an import that reads the whole file anyway, and a scanned file is
hashed once and cached. *Alternative:* a hand-written SHA-256 — a few dozen lines but several times slower and a
security primitive we would own.

**2. The move is a pass in `books_folder`, guarded by a setting.** `books.ids` = `sha256` marks a moved state. When it
is unset, `books_folder` collects every `(old id, path)` from the `file` lines (path in the folder) and the `book`
entries, reads each file once, and maps the old id to the new one only when the file's FNV id still equals the old id
(the file did not change). `Store::rename_ids(&map)` rewrites `book`, `bookmark`, `quote` and `file` lines in one
write; then the setting is written. A fresh state gets the setting on its first start too. Running in `books_folder`
puts the move on the shells' existing background threads and before the scan and the import that would otherwise mix
old and new ids. The measurement flags read the remembered folder without `books_folder`, so they never move anything.

**3. A late rename in the folder scan.** A book removed to the Trash before the update keeps its bookmarks and
highlights under the old id (its file could not be hashed). When the scan parses a new file (a cache miss) it also has
its bytes, so it computes the old id beside the new one and, if the state holds data under the old id, renames it in
the same write as the `file` lines. This costs one FNV pass over at most 128 KiB per newly seen file.

**4. Covers.** Thumbnails are files named by id, so after the move the library sees no thumbnail and makes one in the
background (the existing behaviour for an uncached cover); the old files stay in the cache directory. Generated covers
now match Android's for the same file.

## Risks / Trade-offs

- A state written by this version and opened by 0.11.0 (a downgrade) finds no book by id; positions still come back by
  path, bookmarks and highlights do not until the upgrade. Accepted: downgrades are not supported.
- The first start hashes every book in the folder once: about 500 MB for 500 books, a few seconds on the Pi 5's SD
  card, off the UI thread, while the library waits for its scan. Acceptable once.

## 1. Core

- [x] 1.1 `Book::id` from the content (FNV-1a over length, first and last 64 KiB)
- [x] 1.2 `state::Store`: read, position by id or path, save position with progress, settings, recent books; atomic
      write merged with the file on disk; escaping; damaged lines skipped
- [x] 1.3 `Locator` clamped to a book; progress
- [x] 1.4 Tests: round trip, merge with another writer, damaged file, moved file, clamping; 1000-book timing

## 2. GTK

- [x] 2.1 Store at `user_data_dir/forzac/state`; open at the remembered position; theme and text size from settings
- [x] 2.2 Position changes → save 1 s later and on close; settings saved when changed; measurement flags bypass state
- [x] 2.3 Recent books window when started without files

## 3. macOS

- [x] 3.1 C ABI: store open/close, position, save position, settings
- [x] 3.2 The AppKit shell restores and saves position and settings

## 4. Verification

- [x] 4.1 fmt, clippy, tests on macOS and Linux
- [x] 4.2 Raspberry Pi 5: close and reopen a book on the same line; recent list (screenshot); turns still miss no frames
- [x] 4.3 macOS: reopen restores and saves the position and settings (checked through the state file: the app has no
      screen access for UI automation here)

## Why

The interface strings (help, panels, menus, messages) are English only. Most current Forzac readers read in Russian;
the Android app ships both languages. The maintainer asked for it on 2026-10-17.

## What Changes

- All interface strings in English (default) and Russian, chosen by the system's preferred languages, in both shells:
  the panels and their tabs, the settings sheet, the bars, the pill, the end page, the contents' "Chapter N", the
  recent books, the messages, the help, the macOS menus; the words of the Android app's own strings
  (`values/strings.xml`, `values-ru/strings.xml`), Russian plural forms for counts.
- One catalogue for both shells in the core, keyed by the English text, with a test that every string the shells ask
  for has its Russian and none is left over.
- macOS lists English and Russian as its localisations, so the system's own panels (open, share) follow.
- Hyphenation by the book's language and the language of a book that names none were done in `android-parity`; the
  interface language does not change them.

## Non-Goals

Other languages; a language setting in the app (the system's choice is enough, as on Android); translating key names
in the help; gettext or `.strings` catalogues.

## Capabilities

### New Capabilities
- `localization`: languages, how the language is chosen, what is translated.

### Modified Capabilities
<!-- None: the other specs keep their English words, which this capability translates. -->

## Impact

`core/src/i18n.rs` (the catalogue, plurals, the help texts, the language), a C ABI for the Mac; every visible string
in `gtk/` and `macos/` through `tr`; `macos/Info.plist`.

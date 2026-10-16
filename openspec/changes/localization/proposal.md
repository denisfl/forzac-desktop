## Why

The interface strings (help, panels, menus, messages) are English only. Most current Forzac readers read in Russian;
the Android app ships both languages.

## What Changes

- All interface strings in English (default) and Russian, chosen by the system locale.
- Hyphenation by the book's language; a book without a language gets it detected from its text (as on Android).

## Capabilities

### New Capabilities
- `localization`: languages, how the language is chosen, what is translated.

### Modified Capabilities
<!-- None. -->

## Impact

String catalogues in both shells (gettext on Linux, `.strings` on macOS); language detection in the core.

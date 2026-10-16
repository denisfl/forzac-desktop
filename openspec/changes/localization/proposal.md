## Why

The interface strings (help, panels, menus, messages) are English only. Most current Forzac readers read in Russian;
the Android app ships both languages.

## What Changes

- All interface strings in English (default) and Russian, chosen by the system locale.
- Hyphenation by the book's language (detecting the language of a book that names none moved to `android-parity`).
- The core's "Chapter N" for untitled chapters in the contents (`android-parity`) worded in the interface language.

## Capabilities

### New Capabilities
- `localization`: languages, how the language is chosen, what is translated.

### Modified Capabilities
<!-- None. -->

## Impact

String catalogues in both shells (gettext on Linux, `.strings` on macOS).

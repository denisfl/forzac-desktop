## Why

The spike has text size and theme on single keys. The Android app has an agreed set of preset settings; the desktop
reader needs the same set, reachable from the keyboard, and persistent.

## What Changes

- The agreed settings only: theme (Light, Sepia, Dark, Dawn, Wave, Forest), font (the Android set: Literata, PT Serif,
  Source Serif 4, PT Sans, plus Noto Serif or the closest OFL equivalent), text size (presets), line spacing, alignment
  (justified / left), margins, page mode (from `page-turning`).
- A settings panel on `s`: `j`/`k` choose a setting, `h`/`l` change its value, the page behind re-lays out live;
  `Esc` closes. Direct keys stay: `+`/`−`/`0` size, `t`/`T` theme, `m` mode.
- Light or dark follows the system appearance until the user picks a theme; no separate setting for that.
- Settings are global, persisted through `reading-state`.

No new settings beyond the Android set: following the system appearance is a default, not an option.

## Capabilities

### New Capabilities
- `reader-settings`: the set, the defaults, the panel and keys, what each change costs (redraw or re-layout).

### Modified Capabilities
<!-- None. -->

## Impact

More bundled fonts (OFL, licences in `assets/`); settings in the core; the panel in both shells.

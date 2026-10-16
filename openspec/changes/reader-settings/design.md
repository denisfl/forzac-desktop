## Context

The GTK shell has text size steps (`+`/`−`), six themes (`t`), the page mode (`m`) and remembers them in the reading
state. The core typesets with the bundled PT Serif (four static faces, embedded), a fixed line height of 1.5 and a
column of at most 34 em. The Android app's settings: theme, font (Noto Serif — Android's system serif — Literata,
PT Serif, Source Serif, PT Sans), size, spacing (1.45 / 1.62 / 1.85 em), margins, justified or left, page turning.

## Goals / Non-Goals

**Goals:** the agreed set in the core and the GTK shell, a keyboard panel, live changes, light/dark from the system.

**Non-Goals:** the macOS shell (`macos-app`: it keeps theme and size until then); Noto Serif (Android's system font,
not a bundled one there; the desktop bundles its own); new settings.

## Decisions

### Settings in the core
`forzac_core::settings::Settings` holds the values (theme as an index in the core's theme names, or none for the
system's; size, font, spacing, alignment, margins, page mode), reads and writes them in the state store (keys `theme`,
`text-size`, `font`, `spacing`, `align`, `margins`, `turning`; unknown values fall back to defaults) and makes the
`PageStyle` for a window. Shells keep the colours of the themes. The Mac shell's keys (`theme`, `text-size`) stay valid.

### Font families and variable fonts
`PageStyle` gains a `Family`; the typesetter takes the face's font from the family instead of PT Serif. PT Serif stays
embedded (it is also the base of fallback). Literata and Source Serif 4 are variable fonts (weight and optical size),
PT Sans is four static files, all as shipped by the Android app; they live in `assets/fonts/<family>/` with `OFL.txt`
and are found through `fonts::set_assets(dir)`, set by the shell. A face of a variable family is registered as the file
at a location (`wght` 400 or 700, `opsz` the font's default) — read-fonts builds the instance and HarfRust shapes with
its coordinates — and the shell renders the same location: Pango loads it with the same variations. A missing file
falls back to PT Serif.

The shaper caches each font for its thread, so registered fonts do not take the registry's lock on the hot path.

### The panel (GTK)
A bottom panel with seven rows (setting, value); the selected row is marked, `h`/`l` switch the value and the view
applies the new settings at once (`PageView::set_settings`): a theme only clears the cached nodes, the others re-lay
out with the reading position kept (as a window resize does). Settings are saved on each change.

### Light or dark from the system
With no theme chosen, the shell asks the desktop portal (`org.freedesktop.appearance` `color-scheme`, 1 = prefer dark)
and follows its `SettingChanged` signal; without a portal it uses GTK's `gtk-application-prefer-dark-theme`, else Light.

## Risks / Trade-offs

- Pango choosing a different instance of a variable font than the core shaped → the variations are set explicitly on
  both sides (`wght` and `opsz`), never left to automatic optical sizing.
- A re-layout on every `h`/`l` in the panel costs a chapter's layout (≤ 100 ms on the Pi for a long chapter) → the
  neighbouring layouts are dropped and redone in the background as after a resize.
- The package grows by about 5.6 MB of fonts.

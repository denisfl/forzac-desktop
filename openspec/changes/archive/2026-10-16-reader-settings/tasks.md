## 1. Core

- [x] 1.1 Families: PT Serif embedded; Literata, Source Serif (variable, instances at `wght`/`opsz`), PT Sans from the
      assets directory; fallback to PT Serif; the shaper caches fonts per thread
- [x] 1.2 `PageStyle` with family, spacing, margins and alignment; the typesetter uses the family's faces
- [x] 1.3 `settings::Settings`: values, defaults, reading and writing the state store, the page style for a window
- [x] 1.4 Tests: families shape with their own glyphs and widths, variable bold is wider than regular, settings round
      trip, unknown values fall back

## 2. Assets

- [x] 2.1 Fonts and OFL licences in `assets/fonts/`; packaging and PKGBUILD install them and the licences

## 3. GTK

- [x] 3.1 Fonts by family face with variations in Pango
- [x] 3.2 The view holds `Settings`; theme redraws, the rest re-lays out keeping the position
- [x] 3.3 Settings panel on `s`; `0` default size; settings saved on change
- [x] 3.4 Light or dark from the portal (and its changes) until a theme is chosen
- [x] 3.5 Help and README

## 4. Verification

- [x] 4.1 fmt, clippy, tests on macOS and Linux; the macOS shell still builds and reads its settings
- [x] 4.2 Raspberry Pi 5: each font at each spacing (screenshots), the panel (screenshot), position kept across changes,
      turns still miss no frames

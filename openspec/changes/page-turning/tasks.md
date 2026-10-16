## 1. Core

- [ ] 1.1 Column placement in `paginate`; `flow_height`, `render_flow`, `flow_y`, `flow_locator`
- [ ] 1.2 Tests: the column holds every line once, places round-trip, slices draw the lines in them

## 2. GTK

- [ ] 2.1 Mode in the view; "Slide" unchanged as specified
- [ ] 2.2 "Scroll": tiles per chapter, neighbours above and below, crossing chapters, clamping at the book's ends
- [ ] 2.3 Movement: keys by line, screen and half screen; wheel; touchpad with kinetic glide; nothing drawn when still
- [ ] 2.4 `m` keeps the top line; the mode saved as a setting; navigation, `Tab`, clicks and saving in "Scroll"
- [ ] 2.5 `--scrolls N`; help and README

## 3. Verification

- [ ] 3.1 fmt, clippy, tests on macOS and Linux
- [ ] 3.2 Raspberry Pi 5: `--turns` and `--scrolls` across chapter boundaries at 120 Hz; `m` keeps the line
      (screenshots); idle CPU

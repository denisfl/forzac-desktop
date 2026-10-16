## Why

The desktop app is meant to be the same reader as the Android app, with the keyboard in place of touch. Today it
differs in features and behaviour in ways nobody has listed, so it is not ready to publish, and testing on the Pi 5
and the Mac has nothing to test against.

## What Changes

- A parity list (`docs/android-parity.md`): every reader feature and behaviour of the Android app — opening books,
  typesetting, page turning, contents, notes and links, settings and their presets, themes, reading position, search,
  progress, what happens on errors — with how the desktop app does it.
- Each difference decided with the maintainer: **intended** (keyboard instead of touch, no PDF, no "Curl" and "Cover",
  the items in "Not planned for 1.0") or a **gap**. Items in "Not planned for 1.0" are confirmed or reopened.
- Gaps closed in this change when small, otherwise split into their own changes on the roadmap.
- A by-hand pass on the Pi 5 and the M1 Mac against the list, with the results recorded.

## Capabilities

### New Capabilities
<!-- Decided after the list; gaps modify existing capabilities. -->

### Modified Capabilities
<!-- Decided after the list. -->

## Impact

Documentation first; code changes follow from the gaps.

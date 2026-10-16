# Contributing to Forzac for desktop

Thank you for helping. Forzac is a small, opinionated reader: please read the principles below before proposing a
feature, and open an issue first for anything larger than a fix.

## Principles

- **Few settings, great defaults.** A new setting needs a reason why a good default is not enough. Presets, not sliders.
- **Keyboard first.** Every action has a key; the mouse is a convenience.
- **Typography like a good printed book**: hyphenation, justification, no lone lines at page edges.
- **Fast and quiet.** Nothing is drawn while idle; animations run on the compositor's frame clock; performance claims
  come with measurements on real hardware.

## Layout

```
core/    Rust: model, parsers, hyphenation, shaping, line breaking, pagination → pages as glyph runs
gtk/     Linux shell (GTK 4, Rust)
macos/   macOS shell (AppKit, Swift) over the C ABI in core/src/ffi.rs
assets/  bundled fonts and hyphenation patterns, each with its licence
data/    desktop integration files and packaging
ci/      containers for building and running the GTK shell on other distributions
scripts/ measurements and packaging
```

Architecture and the reasons for it: [docs/adr/0001-stack.md](docs/adr/0001-stack.md).

## Building

Linux (GTK 4.14 or newer, fontconfig, a Rust toolchain from rustup):

```bash
cargo run --release -p forzac-gtk -- book.fb2.zip
```

macOS (Xcode command line tools, Rust with the `aarch64-apple-darwin` target):

```bash
macos/build.sh && open macos/build/Forzac.app --args "$PWD/book.fb2.zip"
```

## Developing for Linux from a Mac

The core builds and tests natively anywhere. The GTK shell can be developed on a Mac with a Raspberry Pi 5 (Raspberry
Pi OS 13 with its desktop) as the Linux machine, no monitor needed:

```bash
PI=user@pi.local scripts/pi-dev.sh book.fb2.zip       # build for aarch64 in Docker, copy, open on the Pi's desktop
PI=user@pi.local scripts/pi-vnc.sh                    # watch the Pi's desktop in macOS Screen Sharing
```

Once on the Pi: `sudo raspi-config nonint do_vnc 0`, then in `/etc/wayvnc/config` set `address=127.0.0.1` and add
`relax_encryption=true`, and `sudo systemctl restart wayvnc`. VNC then listens on the Pi only; `pi-vnc.sh` reaches it
through an SSH tunnel, which also encrypts it. Judge text quality and animation on a real display or with
`scripts/pi-headless.sh`, not over VNC (it compresses the picture).

## Checks

CI runs these on every pull request; run them before pushing:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p forzac-core
```

Tests that need real books read them from a local directory and are skipped without it. Never commit books.

```bash
FORZAC_CORPUS=~/Books cargo test -p forzac-core --test corpus
```

Measurements on a Linux desktop: `scripts/measure.sh book.fb2.zip`; on a machine without a monitor (Raspberry Pi):
`scripts/pi-headless.sh target/release/forzac book.fb2.zip`.

## Process

Features are planned with [OpenSpec](https://github.com/Fission-AI/OpenSpec): a change in `openspec/changes/<name>`
goes through `proposal → specs → design → tasks`, is implemented from its task list and archived when done. The order
of work is in [docs/roadmap.md](docs/roadmap.md). Small fixes need no change.

Everything is written in English: code, comments, docs, commit messages and OpenSpec artefacts.

## Licence

Forzac is licensed under the [Mozilla Public License 2.0](LICENSE). By contributing you agree that your contribution
is licensed under the same terms. There is no contributor licence agreement.

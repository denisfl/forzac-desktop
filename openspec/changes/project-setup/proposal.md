## Why

The spike lives in a local repository without a licence, CI or contributor docs. Before the code is published and
other people send patches, every pull request has to build and test the core and both shells, and the licence has to
be settled: it decides who can reuse the code and how.

## What Changes

- A licence for the code, chosen before the first public commit (options in design: MPL-2.0, MIT/Apache-2.0, GPL-3.0;
  App Store distribution is not planned, so GPL is possible). Bundled fonts and hyphenation patterns keep their own
  licences.
- A public repository with CI on every pull request: `cargo fmt --check`, `cargo clippy -D warnings`, core tests,
  GTK shell builds on Linux x86_64 and aarch64 (the `ci/` containers), the macOS shell on arm64.
- Release artefacts built by CI for tags: Linux binaries with assets for x86_64 and aarch64, the macOS app.
- `CLAUDE.md` / `CONTRIBUTING.md`: product principles, architecture invariants, commands, the OpenSpec process,
  English for everything.
- A local book corpus convention: books are never committed; tests that need them read a directory from an
  environment variable and are skipped without it.

## Capabilities

### New Capabilities
- `project-infrastructure`: what every change must pass (formatting, lints, tests, builds per platform) and how
  releases are produced.

### Modified Capabilities
<!-- None. -->

## Impact

New `.github/workflows/`, `CLAUDE.md`, `CONTRIBUTING.md`, `LICENSE`. No product behaviour changes.

## Non-goals

Signing and notarising the macOS app (in `macos-app`); package repositories (in `linux-integration`).

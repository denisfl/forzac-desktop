## 1. Licence

- [x] 1.1 Add the MPL-2.0 text as `LICENSE`; set `license = "MPL-2.0"` in every crate manifest
- [x] 1.2 Update the README licence section

## 2. Clean checks locally

- [x] 2.1 `cargo fmt --all --check` passes
- [x] 2.2 `cargo clippy --workspace --all-targets -- -D warnings` passes on Linux (GTK shell included) and for the core on macOS
- [x] 2.3 Corpus convention: a helper that returns `FORZAC_CORPUS` books or skips; one corpus test for FB2 parsing

## 3. CI

- [x] 3.1 `.github/workflows/ci.yml`: fmt, clippy, core tests; GTK shell builds on x86_64 and aarch64; macOS shell build
- [x] 3.2 `.github/workflows/release.yml`: archives for Linux x86_64/aarch64 and the macOS app on `v*` tags, draft release
- [x] 3.3 Lint the workflows with actionlint

## 4. Contributor docs

- [x] 4.1 `CONTRIBUTING.md`: building, checks, OpenSpec process, licence of contributions
- [x] 4.2 `CLAUDE.md`: product principles, invariants, commands, working rules

## 5. Verification

- [x] 5.1 Run every CI command locally (macOS for the core and the Mac app, the `ci/` containers for Linux)
- [x] 5.2 Build a Linux archive with the release layout and open a book from it on the Raspberry Pi 5
- [ ] 5.3 After the repository is published: the first pull request shows all checks green (deferred: developing locally until publication, decided 2026-10-16)

## Context

The spike repository builds locally: the core and the GTK shell with cargo (the GTK shell checked in Fedora, Arch and
Debian containers), the macOS shell with `macos/build.sh`. Nothing checks a change automatically, and there is no
licence. The repository is about to be published.

## Goals / Non-Goals

**Goals:** a licence; CI that runs what a developer runs locally, on the three platforms we ship; release archives from
tags; short contributor docs.

**Non-Goals:** signing and notarising the Mac app (`macos-app`); AUR and `.deb` packaging (`linux-integration`);
measuring performance in CI (shared runners have no GPU and noisy timing; measurements stay on the reference hardware).

## Decisions

- **MPL-2.0, one `LICENSE` at the root.** MPL allows the notice to live in a `LICENSE` file instead of per-file headers
  (Exhibit A); crates declare `license = "MPL-2.0"`. No CLA.
- **GitHub Actions, hosted runners.** `ubuntu-24.04` (GTK 4.14), `ubuntu-24.04-arm` for aarch64 (free for public
  repositories), `macos-14` for Apple Silicon. Ubuntu 24.04's GTK 4.14 is the oldest GTK we build against, which keeps
  the minimum honest; newer GTK is covered by the developer containers in `ci/`.
- **Rust toolchain**: stable through `rustup` (`dtolnay/rust-toolchain`), cached with `Swatinem/rust-cache`. The
  repository's `rust-toolchain.toml` says `stable`, so local and CI builds agree.
- **Lints**: `cargo clippy --workspace --all-targets -- -D warnings` on Linux (covers core and GTK shell); the macOS
  job builds the core as a static library and the Swift app.
- **Release**: a separate workflow on `v*` tags builds the three artefacts and attaches them to a draft release; a
  human publishes it. Linux archives use the layout the GTK shell already looks for (`bin/` next to `share/forzac/`).
- **Corpus**: `FORZAC_CORPUS` points to a directory of books; tests skip themselves without it.

## Risks / Trade-offs

- GTK 4.14 on Ubuntu may lack an API a later change wants → that change raises the minimum and says so.
- `ubuntu-24.04-arm` is a hosted runner that GitHub may change → the job is isolated and can move to a container on
  QEMU if needed.
- The unsigned Mac app shows a Gatekeeper warning until `macos-app` adds signing.

## Open Questions

- The GitHub owner and repository name (placeholders in `data/PKGBUILD`).

# Raspberry Pi OS userland (Debian trixie, arm64): build with the distribution's GTK, run under a headless Weston.
#   docker build -t forzac-debian -f ci/debian.Dockerfile ci
#   docker run --rm -v "$PWD":/src -v forzac-debian-target:/target forzac-debian ci/linux-check.sh book.fb2.zip
FROM debian:trixie
RUN apt-get update && apt-get install -y --no-install-recommends libgtk-4-dev libfontconfig-dev pkgconf gcc cargo rustc \
        weston libgl1-mesa-dri mesa-vulkan-drivers procps ca-certificates \
    && rm -rf /var/lib/apt/lists/*
ENV CARGO_TARGET_DIR=/target
WORKDIR /src

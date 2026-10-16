# Raspberry Pi OS userland (Debian trixie, arm64): build against the distribution's GTK, run under a headless Weston.
#   docker build -t forzac-debian -f ci/debian.Dockerfile ci
#   docker run --rm -v "$PWD":/src -v forzac-debian-target:/target forzac-debian ci/linux-check.sh book.fb2.zip
FROM debian:trixie
# Some networks hand out broken .debs over plain HTTP ("Hash Sum mismatch"): fetch over HTTPS. The image has no CA
# bundle yet, so peers are not verified; integrity still comes from the signed Release files apt checks.
RUN sed -i 's|http://|https://|g' /etc/apt/sources.list.d/debian.sources \
    && printf 'Acquire::https::Verify-Peer "false";\nAcquire::Retries "5";\n' > /etc/apt/apt.conf.d/90forzac \
    && apt-get update && apt-get install -y --no-install-recommends libgtk-4-dev libfontconfig-dev pkgconf gcc curl \
        weston libgl1-mesa-dri mesa-vulkan-drivers procps ca-certificates \
    && rm -rf /var/lib/apt/lists/*
# Debian's rustc (1.85) is older than gtk-rs needs (1.92); only the build needs a newer one, the runtime needs GTK.
RUN curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
ENV PATH=/root/.cargo/bin:$PATH CARGO_TARGET_DIR=/target
WORKDIR /src

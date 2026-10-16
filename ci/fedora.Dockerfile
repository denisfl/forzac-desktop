# Linux build and headless run of the GTK shell (also used to measure on CI).
#   docker build -t forzac-fedora -f ci/fedora.Dockerfile ci
#   docker run --rm -v "$PWD":/src -v forzac-target:/target forzac-fedora ci/linux-check.sh book.fb2.zip
FROM fedora:latest
RUN dnf -y install gtk4-devel gcc rust cargo pkgconf-pkg-config fontconfig-devel \
        weston mesa-dri-drivers mesa-vulkan-drivers dbus-daemon procps-ng time \
    && dnf clean all
ENV CARGO_TARGET_DIR=/target
WORKDIR /src

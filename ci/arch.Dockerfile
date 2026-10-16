# Omarchy is Arch: build the package with makepkg and run the app under Hyprland (or sway) in headless mode.
#   docker build --platform linux/amd64 -t forzac-arch -f ci/arch.Dockerfile ci
#   docker run --rm --platform linux/amd64 --cap-add SYS_NICE -v "$PWD":/src forzac-arch /src/ci/arch-check.sh /src/book.fb2.zip
FROM archlinux:latest
# pacman's sandbox needs seccomp, which emulated x86_64 containers lack.
RUN sed -i '/^\[options\]/a DisableSandbox' /etc/pacman.conf \
    && pacman -Syu --noconfirm --needed base-devel git rust gtk4 fontconfig hyprland sway grim wtype \
        xdg-utils perl-file-mimeinfo desktop-file-utils shared-mime-info mesa vulkan-swrast procps-ng python \
    && pacman -Scc --noconfirm
RUN useradd -m builder && echo 'builder ALL=(ALL) NOPASSWD: ALL' > /etc/sudoers.d/builder
USER builder
WORKDIR /home/builder

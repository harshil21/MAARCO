FROM ghcr.io/cross-rs/aarch64-unknown-linux-gnu:latest

# Install ARM64 libudev-dev
RUN dpkg --add-architecture arm64 && \
    apt-get update && \
    apt-get install -y libudev-dev:arm64

# Configure pkg-config for ARM64
ENV PKG_CONFIG_PATH=/usr/lib/aarch64-linux-gnu/pkgconfig
ENV PKG_CONFIG_ALLOW_CROSS=1
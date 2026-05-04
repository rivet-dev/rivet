# Base image for macOS cross-compilation (arm64 + x86_64).
# Pre-bakes osxcross, MacOSX SDK, Rust targets, Node.js, and napi-rs CLI.
#
# Build & push: scripts/docker-builder-base/build-push.sh osxcross
# syntax=docker/dockerfile:1.10.0
FROM rust:1.91.1-bookworm

RUN apt-get update && apt-get install -y \
    git-lfs \
    clang \
    cmake \
    patch \
    libxml2-dev \
    wget \
    xz-utils \
    curl && \
    curl -fsSL https://deb.nodesource.com/setup_22.x | bash - && \
    apt-get install -y nodejs && \
    corepack enable && \
    npm install -g @napi-rs/cli && \
    rm -rf /var/lib/apt/lists/*

# Build osxcross with MacOSX 11.3 SDK
RUN git config --global --add safe.directory '*' && \
    git clone https://github.com/tpoechtrager/osxcross /root/osxcross && \
    cd /root/osxcross && \
    wget -nc https://github.com/phracker/MacOSX-SDKs/releases/download/11.3/MacOSX11.3.sdk.tar.xz && \
    mv MacOSX11.3.sdk.tar.xz tarballs/ && \
    UNATTENDED=yes OSX_VERSION_MIN=10.7 ./build.sh

ENV PATH="/root/osxcross/target/bin:$PATH"

# Install both macOS Rust targets
RUN rustup target add aarch64-apple-darwin x86_64-apple-darwin

# Install sccache (prebuilt musl binary).
RUN SCCACHE_VERSION=v0.8.2 && \
    wget -q https://github.com/mozilla/sccache/releases/download/${SCCACHE_VERSION}/sccache-${SCCACHE_VERSION}-x86_64-unknown-linux-musl.tar.gz && \
    tar -xzf sccache-${SCCACHE_VERSION}-x86_64-unknown-linux-musl.tar.gz && \
    mv sccache-${SCCACHE_VERSION}-x86_64-unknown-linux-musl/sccache /usr/local/bin/sccache && \
    chmod +x /usr/local/bin/sccache && \
    rm -rf sccache-${SCCACHE_VERSION}-x86_64-unknown-linux-musl*

# Shared osxcross env vars
ENV OSXCROSS_SDK=MacOSX11.3.sdk \
    SDKROOT=/root/osxcross/target/SDK/MacOSX11.3.sdk \
    MACOSX_DEPLOYMENT_TARGET=10.14 \
    CARGO_INCREMENTAL=0 \
    CARGO_NET_GIT_FETCH_WITH_CLI=true \
    COREPACK_ENABLE_DOWNLOAD_PROMPT=0

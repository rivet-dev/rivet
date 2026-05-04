# syntax=docker/dockerfile:1.10.0
# Base image for Linux GNU builds (rivetkit-napi addon + rivet-engine).
# Uses Debian bullseye (glibc 2.31) for broad compatibility:
#   Debian 11+, Ubuntu 20.04+, RHEL 9+, Fedora 34+, Amazon Linux 2023+
#
# Pre-bakes Rust, clang 14 (from backports), Node.js 22, napi-rs CLI,
# and the aarch64 cross-compiler.
#
# Build & push: scripts/docker-builder-base/build-push.sh linux-gnu
FROM rust:1.91.1-bullseye

# Install base packages. Bullseye ships clang 11; we pull clang 14 from the
# official LLVM apt repo (https://apt.llvm.org) for modern bindgen support
# (used by *-sys crates like ring, libsqlite3-sys).
RUN apt-get update && apt-get install -y --no-install-recommends \
        git-lfs \
        wget \
        gnupg \
        ca-certificates \
        pkg-config \
        cmake \
        libssl-dev \
        curl && \
    wget -qO- https://apt.llvm.org/llvm-snapshot.gpg.key | gpg --dearmor -o /usr/share/keyrings/llvm.gpg && \
    echo "deb [signed-by=/usr/share/keyrings/llvm.gpg] http://apt.llvm.org/bullseye/ llvm-toolchain-bullseye-14 main" > /etc/apt/sources.list.d/llvm.list && \
    apt-get update && apt-get install -y --no-install-recommends \
        clang-14 \
        llvm-14-dev \
        libclang-14-dev && \
    curl -fsSL https://deb.nodesource.com/setup_22.x | bash - && \
    apt-get install -y --no-install-recommends nodejs && \
    corepack enable && \
    npm install -g @napi-rs/cli && \
    rm -rf /var/lib/apt/lists/*

# Install aarch64 Linux GNU cross-compiler.
RUN apt-get update && apt-get install -y --no-install-recommends \
    gcc-aarch64-linux-gnu \
    g++-aarch64-linux-gnu \
    binutils-aarch64-linux-gnu && \
    rm -rf /var/lib/apt/lists/*

RUN rustup target add x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu

# Install sccache (prebuilt musl binary, runs on glibc too).
RUN SCCACHE_VERSION=v0.8.2 && \
    wget -q https://github.com/mozilla/sccache/releases/download/${SCCACHE_VERSION}/sccache-${SCCACHE_VERSION}-x86_64-unknown-linux-musl.tar.gz && \
    tar -xzf sccache-${SCCACHE_VERSION}-x86_64-unknown-linux-musl.tar.gz && \
    mv sccache-${SCCACHE_VERSION}-x86_64-unknown-linux-musl/sccache /usr/local/bin/sccache && \
    chmod +x /usr/local/bin/sccache && \
    rm -rf sccache-${SCCACHE_VERSION}-x86_64-unknown-linux-musl*

ENV LIBCLANG_PATH=/usr/lib/llvm-14/lib \
    CLANG_PATH=/usr/bin/clang-14 \
    CARGO_INCREMENTAL=0 \
    CARGO_NET_GIT_FETCH_WITH_CLI=true \
    COREPACK_ENABLE_DOWNLOAD_PROMPT=0 \
    CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc \
    CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc \
    CXX_aarch64_unknown_linux_gnu=aarch64-linux-gnu-g++

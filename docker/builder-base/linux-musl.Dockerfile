# syntax=docker/dockerfile:1.10.0
# Base image for Linux static (musl) builds (rivetkit-napi addon + rivet-engine).
# Produces fully static binaries that run on any Linux distro:
#   Alpine, scratch, distroless, Debian, Ubuntu, RHEL, etc.
#
# Hosted on Debian bookworm but cross-compiles via musl-cross toolchains for
# both x86_64 and aarch64. Includes static OpenSSL for both architectures.
# Pre-bakes Rust, Node.js 22, napi-rs CLI.
#
# Build & push: scripts/docker-builder-base/build-push.sh linux-musl
FROM rust:1.89.0-bookworm

RUN apt-get update && apt-get install -y --no-install-recommends \
    musl-tools \
    musl-dev \
    llvm-14-dev \
    libclang-14-dev \
    clang-14 \
    libssl-dev \
    pkg-config \
    ca-certificates \
    g++ \
    g++-multilib \
    git-lfs \
    wget \
    curl && \
    curl -fsSL https://deb.nodesource.com/setup_22.x | bash - && \
    apt-get install -y --no-install-recommends nodejs && \
    corepack enable && \
    npm install -g @napi-rs/cli && \
    rm -rf /var/lib/apt/lists/*

# Install musl cross-compilation toolchains for x86_64 and aarch64.
RUN wget -q https://github.com/cross-tools/musl-cross/releases/latest/download/x86_64-unknown-linux-musl.tar.xz && \
    tar -xf x86_64-unknown-linux-musl.tar.xz -C /opt/ && \
    rm x86_64-unknown-linux-musl.tar.xz

RUN wget -q https://musl.cc/aarch64-linux-musl-cross.tgz && \
    tar -xzf aarch64-linux-musl-cross.tgz -C /opt/ && \
    rm aarch64-linux-musl-cross.tgz

RUN rustup target add x86_64-unknown-linux-musl aarch64-unknown-linux-musl

# Install sccache (prebuilt musl binary).
RUN SCCACHE_VERSION=v0.8.2 && \
    wget -q https://github.com/mozilla/sccache/releases/download/${SCCACHE_VERSION}/sccache-${SCCACHE_VERSION}-x86_64-unknown-linux-musl.tar.gz && \
    tar -xzf sccache-${SCCACHE_VERSION}-x86_64-unknown-linux-musl.tar.gz && \
    mv sccache-${SCCACHE_VERSION}-x86_64-unknown-linux-musl/sccache /usr/local/bin/sccache && \
    chmod +x /usr/local/bin/sccache && \
    rm -rf sccache-${SCCACHE_VERSION}-x86_64-unknown-linux-musl*

# PATH must be set before building OpenSSL so the cross-compilers are found.
ENV PATH="/opt/x86_64-unknown-linux-musl/bin:/opt/aarch64-linux-musl-cross/bin:$PATH"

# Build static OpenSSL for x86_64 musl.
ENV SSL_VER=1.1.1w
RUN wget -q https://www.openssl.org/source/openssl-$SSL_VER.tar.gz && \
    tar -xzf openssl-$SSL_VER.tar.gz && \
    cp -r openssl-$SSL_VER openssl-$SSL_VER-x86_64 && \
    cd openssl-$SSL_VER-x86_64 && \
    CC=x86_64-unknown-linux-musl-gcc \
       ./Configure no-shared no-async --prefix=/musl-x86_64 --openssldir=/musl-x86_64/ssl linux-x86_64 && \
    make -j$(nproc) && \
    make install_sw && \
    cd .. && \
    rm -rf openssl-$SSL_VER-x86_64

# Build static OpenSSL for aarch64 musl.
RUN cp -r openssl-$SSL_VER openssl-$SSL_VER-aarch64 && \
    cd openssl-$SSL_VER-aarch64 && \
    CC=aarch64-linux-musl-gcc \
       ./Configure no-shared no-async no-tests --prefix=/musl-aarch64 --openssldir=/musl-aarch64/ssl linux-aarch64 && \
    make -j$(nproc) build_libs && \
    make install_dev && \
    mkdir -p /musl-aarch64/ssl && \
    cd .. && \
    rm -rf openssl-$SSL_VER-aarch64 openssl-$SSL_VER openssl-$SSL_VER.tar.gz

ENV LIBCLANG_PATH=/usr/lib/llvm-14/lib \
    CLANG_PATH=/usr/bin/clang-14 \
    CARGO_INCREMENTAL=0 \
    CARGO_NET_GIT_FETCH_WITH_CLI=true \
    COREPACK_ENABLE_DOWNLOAD_PROMPT=0 \
    CC_x86_64_unknown_linux_musl=x86_64-unknown-linux-musl-gcc \
    CXX_x86_64_unknown_linux_musl=x86_64-unknown-linux-musl-g++ \
    AR_x86_64_unknown_linux_musl=x86_64-unknown-linux-musl-ar \
    CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER=x86_64-unknown-linux-musl-gcc \
    CC_aarch64_unknown_linux_musl=aarch64-linux-musl-gcc \
    CXX_aarch64_unknown_linux_musl=aarch64-linux-musl-g++ \
    AR_aarch64_unknown_linux_musl=aarch64-linux-musl-ar \
    CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER=aarch64-linux-musl-gcc

# Base image for Linux static (musl) cross-compilation.
# Pre-bakes musl toolchains, OpenSSL, and Node.js.
#
# Build & push: scripts/docker-builder-base/build-push.sh linux-musl
# syntax=docker/dockerfile:1.10.0
FROM rust:1.89.0-bookworm

RUN apt-get update && apt-get install -y \
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
    curl && \
    curl -fsSL https://deb.nodesource.com/setup_22.x | bash - && \
    apt-get install -y nodejs && \
    corepack enable && \
    rm -rf /var/lib/apt/lists/*

# Install musl cross-compilation toolchains
RUN wget -q https://github.com/cross-tools/musl-cross/releases/latest/download/x86_64-unknown-linux-musl.tar.xz && \
    tar -xf x86_64-unknown-linux-musl.tar.xz -C /opt/ && \
    rm x86_64-unknown-linux-musl.tar.xz

RUN wget -q https://musl.cc/aarch64-linux-musl-cross.tgz && \
    tar -xzf aarch64-linux-musl-cross.tgz -C /opt/ && \
    rm aarch64-linux-musl-cross.tgz

RUN rustup target add x86_64-unknown-linux-musl aarch64-unknown-linux-musl

# Build static OpenSSL for x86_64 musl
ENV SSL_VER=1.1.1w
RUN wget https://www.openssl.org/source/openssl-$SSL_VER.tar.gz \
    && tar -xzf openssl-$SSL_VER.tar.gz \
    && cd openssl-$SSL_VER \
    && ./Configure no-shared no-async --prefix=/musl --openssldir=/musl/ssl linux-x86_64 \
    && make -j$(nproc) \
    && make install_sw \
    && cd .. \
    && rm -rf openssl-$SSL_VER*

ENV PATH="/opt/x86_64-unknown-linux-musl/bin:/opt/aarch64-linux-musl-cross/bin:$PATH" \
    LIBCLANG_PATH=/usr/lib/llvm-14/lib \
    CLANG_PATH=/usr/bin/clang-14 \
    CARGO_INCREMENTAL=0 \
    CARGO_NET_GIT_FETCH_WITH_CLI=true \
    COREPACK_ENABLE_DOWNLOAD_PROMPT=0

# syntax=docker/dockerfile:1.10.0
# Base image for Linux engine container builds.
# Pre-bakes Rust, Node.js 22, corepack, build dependencies, and the
# FoundationDB client library for each target architecture.
#
# Build & push: scripts/docker-builder-base/build-push.sh engine-builder --push
FROM mcr.microsoft.com/devcontainers/rust:1-1-bookworm

ARG TARGETARCH

ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update -y && \
    apt-get install -y --no-install-recommends \
    ca-certificates \
    cmake \
    curl \
    g++ \
    git \
    gpg \
    libclang-dev \
    libpq-dev \
    libssl-dev \
    make \
    openssl \
    pkg-config \
    wget && \
    rustup toolchain install 1.91.1 && \
    rustup default 1.91.1 && \
    curl -fsSL https://deb.nodesource.com/setup_22.x | bash - && \
    apt-get install -y --no-install-recommends nodejs && \
    corepack enable && \
    rm -rf /var/lib/apt/lists/* && \
    if [ "$TARGETARCH" = "arm64" ]; then \
        curl -Lf -o /lib/libfdb_c.so "https://github.com/apple/foundationdb/releases/download/7.3.68/libfdb_c.aarch64.so"; \
    else \
        curl -Lf -o /lib/libfdb_c.so "https://github.com/apple/foundationdb/releases/download/7.3.68/libfdb_c.x86_64.so"; \
    fi

ENV CARGO_NET_GIT_FETCH_WITH_CLI=true \
    COREPACK_ENABLE_DOWNLOAD_PROMPT=0

WORKDIR /app

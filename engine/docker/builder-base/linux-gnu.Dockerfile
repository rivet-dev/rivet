# Base image for Linux GNU native builds (napi-rs).
# Pre-bakes clang, Node.js, and napi-rs CLI.
#
# Build & push: scripts/docker-builder-base/build-push.sh linux-gnu
# syntax=docker/dockerfile:1.10.0
FROM rust:1.89.0-bookworm

RUN apt-get update && apt-get install -y \
    git-lfs \
    clang \
    llvm-dev \
    libclang-dev \
    curl && \
    curl -fsSL https://deb.nodesource.com/setup_22.x | bash - && \
    apt-get install -y nodejs && \
    npm install -g @napi-rs/cli && \
    rm -rf /var/lib/apt/lists/*

ENV CARGO_INCREMENTAL=0 \
    CARGO_NET_GIT_FETCH_WITH_CLI=true \
    COREPACK_ENABLE_DOWNLOAD_PROMPT=0

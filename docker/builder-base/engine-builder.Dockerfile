# syntax=docker/dockerfile:1.10.0
# Base image for Linux engine container builds.
# Pre-bakes Rust and build dependencies for each target architecture.
#
# Build & push: scripts/docker-builder-base/build-push.sh engine-builder --push
FROM mcr.microsoft.com/devcontainers/rust:1-1-bookworm

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
    rm -rf /var/lib/apt/lists/*

ENV CARGO_NET_GIT_FETCH_WITH_CLI=true

WORKDIR /app

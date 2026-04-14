# syntax=docker/dockerfile:1.10.0
# Base image for the full Linux engine runtime image.
#
# Build & push: scripts/docker-builder-base/build-push.sh engine-runtime-full --push
FROM mcr.microsoft.com/devcontainers/base:debian

ARG TARGETARCH

ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update -y && \
    apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    dirmngr \
    gpg \
    openssl && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/* && \
    if [ "$TARGETARCH" = "arm64" ]; then \
        curl -Lf -o /lib/libfdb_c.so "https://github.com/apple/foundationdb/releases/download/7.3.68/libfdb_c.aarch64.so"; \
    else \
        curl -Lf -o /lib/libfdb_c.so "https://github.com/apple/foundationdb/releases/download/7.3.68/libfdb_c.x86_64.so"; \
    fi

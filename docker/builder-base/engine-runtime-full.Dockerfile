# syntax=docker/dockerfile:1.10.0
# Base image for the full Linux engine runtime image.
#
# Build & push: scripts/docker-builder-base/build-push.sh engine-runtime-full --push
FROM mcr.microsoft.com/devcontainers/base:debian

ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update -y && \
    apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    dirmngr \
    gpg \
    openssl && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

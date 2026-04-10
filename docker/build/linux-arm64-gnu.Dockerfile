# syntax=docker/dockerfile:1.10.0
# Unified build for linux-arm64-gnu (cross-compiled from x86_64).
# See linux-x64-gnu.Dockerfile for build arg documentation.
#
# Base image: docker/builder-base/linux-gnu.Dockerfile
ARG BASE_TAG=latest
FROM ghcr.io/rivet-dev/rivet/builder-base-linux-gnu:${BASE_TAG}

ARG BUILD_TARGET=engine
ARG BUILD_MODE=release
ARG BUILD_FRONTEND=false
ARG VITE_APP_API_URL=__SAME__

ENV RUSTFLAGS="--cfg tokio_unstable"

WORKDIR /build
COPY . .

RUN if [ "$BUILD_TARGET" = "engine" ] && [ "$BUILD_FRONTEND" = "true" ]; then \
        export NODE_OPTIONS="--max-old-space-size=8192" && \
        pnpm install && \
        if [ -n "$VITE_APP_API_URL" ]; then \
            VITE_APP_API_URL="${VITE_APP_API_URL}" npx turbo build:engine -F @rivetkit/engine-frontend; \
        else \
            npx turbo build:engine -F @rivetkit/engine-frontend; \
        fi; \
    fi

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/build/target \
    set -e && \
    if [ "$BUILD_MODE" = "release" ]; then \
        CARGO_FLAG="--release"; \
        PROFILE_DIR="release"; \
    else \
        CARGO_FLAG=""; \
        PROFILE_DIR="debug"; \
    fi && \
    mkdir -p /artifacts && \
    if [ "$BUILD_TARGET" = "engine" ]; then \
        cargo build --bin rivet-engine $CARGO_FLAG --target aarch64-unknown-linux-gnu && \
        cp target/aarch64-unknown-linux-gnu/$PROFILE_DIR/rivet-engine /artifacts/rivet-engine-aarch64-unknown-linux-gnu; \
    elif [ "$BUILD_TARGET" = "rivetkit-native" ]; then \
        cd rivetkit-typescript/packages/rivetkit-native && \
        napi build --platform $CARGO_FLAG --target aarch64-unknown-linux-gnu && \
        cp rivetkit-native.linux-arm64-gnu.node /artifacts/; \
    else \
        echo "Unknown BUILD_TARGET: $BUILD_TARGET" && exit 1; \
    fi

CMD ["ls", "-la", "/artifacts"]

# syntax=docker/dockerfile:1.10.0
# Unified build for linux-arm64-musl (fully static, cross-compiled).
# See linux-x64-gnu.Dockerfile for build arg documentation.
#
# Base image: docker/builder-base/linux-musl.Dockerfile
ARG BASE_TAG=latest
FROM ghcr.io/rivet-dev/rivet/builder-base-linux-musl:${BASE_TAG}

ARG BUILD_TARGET=engine
ARG BUILD_MODE=release
ARG BUILD_FRONTEND=false
ARG VITE_APP_API_URL=__SAME__

ENV OPENSSL_DIR=/musl-aarch64 \
    OPENSSL_INCLUDE_DIR=/musl-aarch64/include \
    OPENSSL_LIB_DIR=/musl-aarch64/lib \
    OPENSSL_STATIC=1 \
    PKG_CONFIG_ALLOW_CROSS=1


ENV RUSTC_WRAPPER=sccache \
    SCCACHE_WEBDAV_ENDPOINT=https://cache.depot.dev \
    SCCACHE_IDLE_TIMEOUT=0

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

RUN --mount=type=cache,id=cargo-registry-linux-arm64-musl,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=cargo-git-linux-arm64-musl,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,id=cargo-target-linux-arm64-musl,target=/build/target,sharing=locked \
    --mount=type=secret,id=DEPOT_TOKEN,env=SCCACHE_WEBDAV_TOKEN \
    set -e && \
    if [ -z "${SCCACHE_WEBDAV_TOKEN:-}" ]; then \
        echo "[sccache] no DEPOT_TOKEN, disabling"; unset RUSTC_WRAPPER; \
    elif ! (sccache --start-server 2>/tmp/sccache-start.err && sccache --show-stats >/dev/null 2>&1); then \
        echo "[sccache] backend health check failed, disabling:"; cat /tmp/sccache-start.err 2>/dev/null || true; \
        sccache --stop-server >/dev/null 2>&1 || true; \
        unset RUSTC_WRAPPER SCCACHE_WEBDAV_ENDPOINT SCCACHE_WEBDAV_TOKEN; \
    else \
        echo "[sccache] enabled via ${SCCACHE_WEBDAV_ENDPOINT}"; \
    fi && \
    if [ "$BUILD_MODE" = "release" ]; then \
        CARGO_FLAG="--release"; \
        PROFILE_DIR="release"; \
    else \
        CARGO_FLAG=""; \
        PROFILE_DIR="debug"; \
    fi && \
    mkdir -p /artifacts && \
    if [ "$BUILD_TARGET" = "engine" ]; then \
        RUSTFLAGS="--cfg tokio_unstable -C target-feature=+crt-static -C link-arg=-static-libgcc" \
            cargo build --bin rivet-engine $CARGO_FLAG --target aarch64-unknown-linux-musl && \
        cp target/aarch64-unknown-linux-musl/$PROFILE_DIR/rivet-engine /artifacts/rivet-engine-aarch64-unknown-linux-musl; \
    elif [ "$BUILD_TARGET" = "rivetkit-native" ]; then \
        cd rivetkit-typescript/packages/rivetkit-native && \
        RUSTFLAGS="--cfg tokio_unstable -C target-feature=-crt-static" \
            napi build --platform $CARGO_FLAG --target aarch64-unknown-linux-musl && \
        cp rivetkit-native.linux-arm64-musl.node /artifacts/; \
    else \
        echo "Unknown BUILD_TARGET: $BUILD_TARGET" && exit 1; \
    fi && \
    (sccache --show-stats 2>/dev/null || true)

CMD ["ls", "-la", "/artifacts"]

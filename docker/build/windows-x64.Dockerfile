# syntax=docker/dockerfile:1.10.0
# Unified build for windows-x64 (MinGW cross-compile).
# See linux-x64-gnu.Dockerfile for build arg documentation.
#
# NOTE on MinGW vs MSVC: rivetkit-native and rivet-engine both use MinGW on
# Windows to share a single Docker base image. MSVC would match Node.js's
# own build toolchain more precisely, but cross-compiling MSVC from Linux
# requires cargo-xwin and a separate base image. MinGW-built .node files load
# into MSVC Node.js in practice as long as we statically link libgcc/libstdc++.
#
# Base image: docker/builder-base/windows-mingw.Dockerfile
ARG BASE_TAG=latest
FROM ghcr.io/rivet-dev/rivet/builder-base-windows-mingw:${BASE_TAG}

ARG BUILD_TARGET=engine
ARG BUILD_MODE=release
ARG BUILD_FRONTEND=false
ARG VITE_APP_API_URL=__SAME__

# Windows-specific build flags:
# - lld linker is ~5x faster than MinGW's default ld for big Rust binaries.
# - static-libgcc avoids runtime libgcc_s_seh-1.dll dependency.
# - static-libstdc++ dropped: our code has no C++ (only Rust + C), and static
#   libstdc++ adds 1-2 min of link time for no benefit.
ENV RUSTFLAGS="--cfg tokio_unstable -C target-feature=+crt-static -C link-arg=-static-libgcc -C link-arg=-fuse-ld=lld"

ENV RUSTC_WRAPPER=sccache \
    SCCACHE_WEBDAV_ENDPOINT=https://cache.depot.dev

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
    --mount=type=secret,id=DEPOT_TOKEN,env=SCCACHE_WEBDAV_TOKEN \
    set -e && \
    if [ -z "$SCCACHE_WEBDAV_TOKEN" ]; then echo "[sccache] no DEPOT_TOKEN, disabling sccache"; unset RUSTC_WRAPPER; fi && \
    if [ "$BUILD_MODE" = "release" ]; then \
        CARGO_FLAG="--release"; \
        PROFILE_DIR="release"; \
    else \
        CARGO_FLAG=""; \
        PROFILE_DIR="debug"; \
    fi && \
    mkdir -p /artifacts && \
    if [ "$BUILD_TARGET" = "engine" ]; then \
        cargo build --bin rivet-engine $CARGO_FLAG --target x86_64-pc-windows-gnu && \
        cp target/x86_64-pc-windows-gnu/$PROFILE_DIR/rivet-engine.exe /artifacts/rivet-engine-x86_64-pc-windows-gnu.exe; \
    elif [ "$BUILD_TARGET" = "rivetkit-native" ]; then \
        cd rivetkit-typescript/packages/rivetkit-native && \
        napi build --platform $CARGO_FLAG --target x86_64-pc-windows-gnu && \
        # napi-rs names the output after the build host's naming convention.
        # The runtime loader expects .win32-x64-msvc.node, so rename if needed.
        if [ -f rivetkit-native.win32-x64-gnu.node ]; then \
            cp rivetkit-native.win32-x64-gnu.node /artifacts/rivetkit-native.win32-x64-msvc.node; \
        else \
            cp rivetkit-native.win32-x64-msvc.node /artifacts/; \
        fi; \
    else \
        echo "Unknown BUILD_TARGET: $BUILD_TARGET" && exit 1; \
    fi

CMD ["ls", "-la", "/artifacts"]

# syntax=docker/dockerfile:1.10.0
# Unified build for darwin-arm64 (Apple Silicon) via osxcross.
# See linux-x64-gnu.Dockerfile for build arg documentation.
#
# Base image: engine/docker/builder-base/osxcross.Dockerfile
ARG BASE_TAG=latest
FROM ghcr.io/rivet-dev/rivet/builder-base-osxcross:${BASE_TAG}

ARG BUILD_TARGET=engine
ARG BUILD_MODE=release
ARG BUILD_FRONTEND=false
ARG VITE_APP_API_URL=__SAME__

ENV BINDGEN_EXTRA_CLANG_ARGS_aarch64_apple_darwin="--sysroot=/root/osxcross/target/SDK/MacOSX11.3.sdk -isystem /root/osxcross/target/SDK/MacOSX11.3.sdk/usr/include" \
    CFLAGS_aarch64_apple_darwin="-B/root/osxcross/target/bin" \
    CXXFLAGS_aarch64_apple_darwin="-B/root/osxcross/target/bin" \
    CARGO_TARGET_AARCH64_APPLE_DARWIN_LINKER=aarch64-apple-darwin20.4-clang \
    CC_aarch64_apple_darwin=aarch64-apple-darwin20.4-clang \
    CXX_aarch64_apple_darwin=aarch64-apple-darwin20.4-clang++ \
    AR_aarch64_apple_darwin=aarch64-apple-darwin20.4-ar \
    RANLIB_aarch64_apple_darwin=aarch64-apple-darwin20.4-ranlib \
    RUSTFLAGS="--cfg tokio_unstable"

RUN mkdir -p /root/.cargo && \
    echo '[target.aarch64-apple-darwin]\nlinker = "aarch64-apple-darwin20.4-clang"\nar = "aarch64-apple-darwin20.4-ar"\n' > /root/.cargo/config.toml

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
        cargo build --bin rivet-engine $CARGO_FLAG --target aarch64-apple-darwin && \
        cp target/aarch64-apple-darwin/$PROFILE_DIR/rivet-engine /artifacts/rivet-engine-aarch64-apple-darwin; \
    elif [ "$BUILD_TARGET" = "rivetkit-native" ]; then \
        cd rivetkit-typescript/packages/rivetkit-native && \
        NAPI_RS_CROSS_COMPILE=1 napi build --platform $CARGO_FLAG --target aarch64-apple-darwin && \
        cp rivetkit-native.darwin-arm64.node /artifacts/; \
    else \
        echo "Unknown BUILD_TARGET: $BUILD_TARGET" && exit 1; \
    fi

CMD ["ls", "-la", "/artifacts"]

# syntax=docker/dockerfile:1.10.0
# Unified build for darwin-x64 (Intel Mac) via osxcross.
# See linux-x64-gnu.Dockerfile for build arg documentation.
#
# Base image: docker/builder-base/osxcross.Dockerfile
FROM ghcr.io/rivet-dev/rivet/builder-base-osxcross:0e33ceb98

ARG BUILD_TARGET=engine
ARG BUILD_MODE=release
ARG BUILD_FRONTEND=false
ARG VITE_APP_API_URL=__SAME__

ENV BINDGEN_EXTRA_CLANG_ARGS_x86_64_apple_darwin="--sysroot=/root/osxcross/target/SDK/MacOSX11.3.sdk -isystem /root/osxcross/target/SDK/MacOSX11.3.sdk/usr/include" \
    CFLAGS_x86_64_apple_darwin="-B/root/osxcross/target/bin" \
    CXXFLAGS_x86_64_apple_darwin="-B/root/osxcross/target/bin" \
    CARGO_TARGET_X86_64_APPLE_DARWIN_LINKER=x86_64-apple-darwin20.4-clang \
    CC_x86_64_apple_darwin=x86_64-apple-darwin20.4-clang \
    CXX_x86_64_apple_darwin=x86_64-apple-darwin20.4-clang++ \
    AR_x86_64_apple_darwin=x86_64-apple-darwin20.4-ar \
    RANLIB_x86_64_apple_darwin=x86_64-apple-darwin20.4-ranlib \
    RUSTFLAGS="--cfg tokio_unstable"

RUN mkdir -p /root/.cargo && \
    echo '[target.x86_64-apple-darwin]\nlinker = "x86_64-apple-darwin20.4-clang"\nar = "x86_64-apple-darwin20.4-ar"\n' > /root/.cargo/config.toml


ENV RUSTC_WRAPPER=sccache \
    SCCACHE_WEBDAV_ENDPOINT=https://cache.depot.dev \
    SCCACHE_IDLE_TIMEOUT=0

WORKDIR /build
COPY . .

RUN if [ "$BUILD_TARGET" = "engine" ] && [ "$BUILD_FRONTEND" = "true" ]; then \
        export NODE_OPTIONS="--max-old-space-size=8192" && \
        export SKIP_NAPI_BUILD=1 && \
        pnpm install --ignore-scripts && \
        if [ -n "$VITE_APP_API_URL" ]; then \
            VITE_APP_API_URL="${VITE_APP_API_URL}" npx turbo build:engine -F @rivetkit/engine-frontend; \
        else \
            npx turbo build:engine -F @rivetkit/engine-frontend; \
        fi; \
    fi

RUN --mount=type=cache,id=cargo-registry-darwin-x64,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=cargo-git-darwin-x64,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,id=cargo-target-darwin-x64,target=/build/target,sharing=locked \
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
        cargo build --bin rivet-engine $CARGO_FLAG --target x86_64-apple-darwin && \
        cp target/x86_64-apple-darwin/$PROFILE_DIR/rivet-engine /artifacts/rivet-engine-x86_64-apple-darwin; \
    elif [ "$BUILD_TARGET" = "rivetkit-napi" ]; then \
        cd rivetkit-typescript/packages/rivetkit-napi && \
        NAPI_RS_CROSS_COMPILE=1 napi build --platform $CARGO_FLAG --target x86_64-apple-darwin && \
        cp rivetkit-napi.darwin-x64.node /artifacts/; \
    else \
        echo "Unknown BUILD_TARGET: $BUILD_TARGET" && exit 1; \
    fi && \
    (sccache --show-stats 2>/dev/null || true)

CMD ["ls", "-la", "/artifacts"]

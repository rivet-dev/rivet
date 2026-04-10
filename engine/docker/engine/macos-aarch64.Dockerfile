# syntax=docker/dockerfile:1.10.0
# Base image built from: engine/docker/builder-base/osxcross.Dockerfile
# Rebuild base: scripts/docker-builder-base/build-push.sh osxcross --push
FROM ghcr.io/rivet-dev/rivet/builder-base-osxcross:TODO AS base

ARG BUILD_FRONTEND=true
ARG VITE_APP_API_URL=__SAME__

# aarch64-specific cross-compilation env
ENV BINDGEN_EXTRA_CLANG_ARGS_aarch64_apple_darwin="--sysroot=/root/osxcross/target/SDK/MacOSX11.3.sdk -isystem /root/osxcross/target/SDK/MacOSX11.3.sdk/usr/include" \
    CFLAGS_aarch64_apple_darwin="-B/root/osxcross/target/bin" \
    CXXFLAGS_aarch64_apple_darwin="-B/root/osxcross/target/bin" \
    CARGO_TARGET_AARCH64_APPLE_DARWIN_LINKER=aarch64-apple-darwin20.4-clang \
    CC_aarch64_apple_darwin=aarch64-apple-darwin20.4-clang \
    CXX_aarch64_apple_darwin=aarch64-apple-darwin20.4-clang++ \
    AR_aarch64_apple_darwin=aarch64-apple-darwin20.4-ar \
    RANLIB_aarch64_apple_darwin=aarch64-apple-darwin20.4-ranlib \
    RUSTFLAGS="--cfg tokio_unstable"

WORKDIR /build

FROM base AS aarch64-builder

RUN mkdir -p /root/.cargo && \
    echo '[target.aarch64-apple-darwin]\nlinker = "aarch64-apple-darwin20.4-clang"\nar = "aarch64-apple-darwin20.4-ar"\n' > /root/.cargo/config.toml

COPY . .

RUN if [ "$BUILD_FRONTEND" = "true" ]; then \
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
    cargo build --bin rivet-engine --release --target aarch64-apple-darwin && \
    mkdir -p /artifacts && \
    cp target/aarch64-apple-darwin/release/rivet-engine /artifacts/rivet-engine-aarch64-apple-darwin

CMD ["ls", "-la", "/artifacts"]

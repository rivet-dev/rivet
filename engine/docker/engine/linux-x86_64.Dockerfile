# syntax=docker/dockerfile:1.10.0
# Base image built from: engine/docker/builder-base/linux-musl.Dockerfile
# Rebuild base: scripts/docker-builder-base/build-push.sh linux-musl --push
FROM ghcr.io/rivet-dev/rivet/builder-base-linux-musl:TODO AS base

ARG BUILD_FRONTEND=true
ARG VITE_APP_API_URL=__SAME__

ENV CC_x86_64_unknown_linux_musl=x86_64-unknown-linux-musl-gcc \
    CXX_x86_64_unknown_linux_musl=x86_64-unknown-linux-musl-g++ \
    AR_x86_64_unknown_linux_musl=x86_64-unknown-linux-musl-ar \
    CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER=x86_64-unknown-linux-musl-gcc \
    RUSTFLAGS="--cfg tokio_unstable -C target-feature=+crt-static -C link-arg=-static-libgcc"

WORKDIR /build

FROM base AS x86_64-builder

ENV OPENSSL_DIR=/musl \
    OPENSSL_INCLUDE_DIR=/musl/include \
    OPENSSL_LIB_DIR=/musl/lib \
    PKG_CONFIG_ALLOW_CROSS=1

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
    cargo build --bin rivet-engine --release --target x86_64-unknown-linux-musl -v && \
    mkdir -p /artifacts && \
    cp target/x86_64-unknown-linux-musl/release/rivet-engine /artifacts/rivet-engine-x86_64-unknown-linux-musl

CMD ["ls", "-la", "/artifacts"]

# syntax=docker/dockerfile:1.10.0
# Base image built from: engine/docker/builder-base/linux-musl.Dockerfile
# Rebuild base: scripts/docker-builder-base/build-push.sh linux-musl --push
FROM ghcr.io/rivet-dev/rivet/builder-base-linux-musl:TODO AS base

ARG BUILD_FRONTEND=true
ARG VITE_APP_API_URL=__SAME__

ENV CC_aarch64_unknown_linux_musl=aarch64-unknown-linux-musl-gcc \
    CXX_aarch64_unknown_linux_musl=aarch64-unknown-linux-musl-g++ \
    AR_aarch64_unknown_linux_musl=aarch64-unknown-linux-musl-ar \
    CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER=aarch64-unknown-linux-musl-gcc \
    RUSTFLAGS="--cfg tokio_unstable -C target-feature=+crt-static -C link-arg=-static-libgcc"

WORKDIR /build

FROM base AS aarch64-builder

# Build static OpenSSL for aarch64 musl
ENV SSL_VER=1.1.1w
RUN wget https://www.openssl.org/source/openssl-$SSL_VER.tar.gz \
    && tar -xzf openssl-$SSL_VER.tar.gz \
    && cd openssl-$SSL_VER \
    && CC=aarch64-unknown-linux-musl-gcc \
       ./Configure no-shared no-async no-tests --prefix=/musl-aarch64 --openssldir=/musl-aarch64/ssl linux-aarch64 \
    && make -j$(nproc) build_libs \
    && make install_dev \
    && mkdir -p /musl-aarch64/ssl \
    && cd .. \
    && rm -rf openssl-$SSL_VER*

ENV OPENSSL_DIR=/musl-aarch64 \
    OPENSSL_INCLUDE_DIR=/musl-aarch64/include \
    OPENSSL_LIB_DIR=/musl-aarch64/lib \
    OPENSSL_STATIC=1 \
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
    cargo build --bin rivet-engine --release --target aarch64-unknown-linux-musl -v && \
    mkdir -p /artifacts && \
    cp target/aarch64-unknown-linux-musl/release/rivet-engine /artifacts/rivet-engine-aarch64-unknown-linux-musl

CMD ["ls", "-la", "/artifacts"]

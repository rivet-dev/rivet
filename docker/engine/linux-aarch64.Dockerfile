# syntax=docker/dockerfile:1.4
FROM rust:1.88.0 AS base

ARG BUILD_FRONTEND=true
ARG VITE_APP_API_URL

# Install dependencies
RUN apt-get update && apt-get install -y \
    musl-tools \
    musl-dev \
    llvm-14-dev \
    libclang-14-dev \
    clang-14 \
    libssl-dev \
    pkg-config \
    protobuf-compiler \
    ca-certificates \
    g++ \
    g++-multilib \
    git-lfs \
    curl && \
    curl -fsSL https://deb.nodesource.com/setup_22.x | bash - && \
    apt-get install -y nodejs && \
    corepack enable && \
    rm -rf /var/lib/apt/lists/* && \
    wget -q https://github.com/cross-tools/musl-cross/releases/download/20250815/aarch64-unknown-linux-musl.tar.xz && \
    tar -xzf aarch64-unknown-linux-musl.tgz -C /opt/ && \
    rm aarch64-unknown-linux-musl.tgz

# Disable interactive prompt
ENV COREPACK_ENABLE_DOWNLOAD_PROMPT=0

# Install musl targets
RUN rustup target add aarch64-unknown-linux-musl

# Set environment variables
ENV PATH="/opt/aarch64-unknown-linux-musl/bin:$PATH" \
    LIBCLANG_PATH=/usr/lib/llvm-14/lib \
    CLANG_PATH=/usr/bin/clang-14 \
    CC_aarch64_unknown_linux_musl=aarch64-unknown-linux-musl-gcc \
    CXX_aarch64_unknown_linux_musl=aarch64-unknown-linux-musl-g++ \
    AR_aarch64_unknown_linux_musl=aarch64-unknown-linux-musl-ar \
    CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER=aarch64-unknown-linux-musl-gcc \
    CARGO_INCREMENTAL=0 \
    RUSTFLAGS="--cfg tokio_unstable -C target-feature=+crt-static -C link-arg=-static-libgcc" \
    CARGO_NET_GIT_FETCH_WITH_CLI=true

# Set working directory
WORKDIR /build

# Build for aarch64
FROM base AS aarch64-builder

# Set up OpenSSL for aarch64 musl target
ENV SSL_VER=1.1.1w
RUN wget https://www.openssl.org/source/openssl-$SSL_VER.tar.gz \
    && tar -xzf openssl-$SSL_VER.tar.gz \
    && cd openssl-$SSL_VER \
    && ./Configure no-shared no-async --prefix=/musl-aarch64 --openssldir=/musl-aarch64/ssl linux-aarch64 \
    && make -j$(nproc) \
    && make install_sw \
    && cd .. \
    && rm -rf openssl-$SSL_VER*

# Configure OpenSSL env vars for the build
ENV OPENSSL_DIR=/musl-aarch64 \
    OPENSSL_INCLUDE_DIR=/musl-aarch64/include \
    OPENSSL_LIB_DIR=/musl-aarch64/lib \
    PKG_CONFIG_ALLOW_CROSS=1

# Copy the source code
COPY . .

# Build frontend
RUN if [ "$BUILD_FRONTEND" = "true" ]; then \
        (cd sdks/typescript/api-full && pnpm install && pnpm run build) && \
        (cd frontend && pnpm install && \
        if [ -n "$VITE_APP_API_URL" ]; then \
            VITE_APP_API_URL="${VITE_APP_API_URL}" pnpm run build:engine; \
        else \
            pnpm run build:engine; \
        fi); \
    fi

# Build for Linux with musl (static binary) - aarch64
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/build/target \
    cargo build --bin rivet-engine --release --target aarch64-unknown-linux-musl -v && \
    mkdir -p /artifacts && \
    cp target/aarch64-unknown-linux-musl/release/rivet-engine /artifacts/rivet-engine-aarch64-unknown-linux-musl

# Default command to show help
CMD ["ls", "-la", "/artifacts"]
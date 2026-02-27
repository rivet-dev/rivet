# syntax=docker/dockerfile:1.10.0

# MARK: Builder
# TODO(RVT-4168): Compile libfdb from scratch for ARM
FROM rust:1.91.0-trixie AS builder

# Docker automatically provides TARGETARCH
ARG TARGETARCH

ARG BUILD_FRONTEND=false
ARG CARGO_BUILD_MODE=debug
ARG VITE_APP_API_URL=__SAME__
ARG OVERRIDE_GIT_SHA

ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update -y && \
    apt-get install -y \
    curl \
    g++ \
    git \
    libclang-dev \
    libpq-dev \
    libssl-dev \
    pkg-config \
    ca-certificates \
    gpg \
    openssl \
    wget \
    cmake \
    make && \
    curl -fsSL https://deb.nodesource.com/setup_22.x | bash - && \
    apt-get install -y nodejs && \
    corepack enable && \
    if [ "$TARGETARCH" = "arm64" ]; then \
        curl -Lf -o /lib/libfdb_c.so "https://github.com/apple/foundationdb/releases/download/7.3.68/libfdb_c.aarch64.so"; \
    else \
        curl -Lf -o /lib/libfdb_c.so "https://github.com/apple/foundationdb/releases/download/7.3.68/libfdb_c.x86_64.so"; \
    fi

# Disable interactive prompt
ENV COREPACK_ENABLE_DOWNLOAD_PROMPT=0

# Pull via Git CLI to improve reliability in CI
ENV CARGO_NET_GIT_FETCH_WITH_CLI=true

WORKDIR /app

COPY . .

# Build frontend
RUN if [ "$BUILD_FRONTEND" = "true" ]; then \
        export NODE_OPTIONS="--max-old-space-size=8192" && \
        pnpm install && \
        if [ -n "$VITE_APP_API_URL" ]; then \
            VITE_APP_API_URL="${VITE_APP_API_URL}" npx turbo build:engine -F @rivetkit/engine-frontend; \
        else \
            npx turbo build:engine -F @rivetkit/engine-frontend; \
        fi; \
    fi

# Build and copy all binaries from target directory into an empty image (it is not
# included in the output because of cache mount)
RUN \
    --mount=type=secret,id=netrc,target=/root/.netrc,mode=0600 \
    --mount=type=cache,target=/usr/local/cargo/git,id=univseral-cargo-git \
    --mount=type=cache,target=/usr/local/cargo/registry,id=univseral-cargo-registry \
    --mount=type=cache,target=/app/target,id=univseral-target \
    --mount=type=cache,target=/root/.cache,id=universal-user-cache \
    if [ "$CARGO_BUILD_MODE" = "release" ]; then \
        RUSTFLAGS="--cfg tokio_unstable" cargo build --bin rivet-engine --release; \
    else \
        RUSTFLAGS="--cfg tokio_unstable" cargo build --bin rivet-engine; \
    fi && \
    # cargo install --locked tokio-console && \
    mkdir /app/dist/ && \
    echo "Copying binary" && \
    cp target/$CARGO_BUILD_MODE/rivet-engine /app/dist/

# MARK: Engine (full, base)
FROM debian:13.1-slim AS engine-full-base

# Docker automatically provides TARGETARCH
ARG TARGETARCH

ENV DEBIAN_FRONTEND=noninteractive
# - Install curl for health checks
RUN apt-get update -y && \
    apt-get install -y \
    ca-certificates \
    openssl \
    curl \
    gpg \
    dirmngr && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/* && \
    if [ "$TARGETARCH" = "arm64" ]; then \
        curl -Lf -o /lib/libfdb_c.so "https://github.com/apple/foundationdb/releases/download/7.3.68/libfdb_c.aarch64.so"; \
    else \
        curl -Lf -o /lib/libfdb_c.so "https://github.com/apple/foundationdb/releases/download/7.3.68/libfdb_c.x86_64.so"; \
    fi

# MARK: Engine (Full)
FROM engine-full-base AS engine-full

LABEL org.opencontainers.image.source=https://github.com/rivet-gg/rivet

COPY --from=builder /app/dist/rivet-engine /usr/bin/rivet-engine

ENTRYPOINT ["/usr/bin/rivet-engine"]
CMD ["start"]

# MARK: Engine (Slim)
FROM debian:13.1-slim AS engine-slim

LABEL org.opencontainers.image.source=https://github.com/rivet-gg/rivet

# Docker automatically provides TARGETARCH
ARG TARGETARCH

ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update -y && \
    apt-get install -y ca-certificates openssl curl && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/* && \
    if [ "$TARGETARCH" = "arm64" ]; then \
        curl -Lf -o /lib/libfdb_c.so "https://github.com/apple/foundationdb/releases/download/7.3.68/libfdb_c.aarch64.so"; \
    else \
        curl -Lf -o /lib/libfdb_c.so "https://github.com/apple/foundationdb/releases/download/7.3.68/libfdb_c.x86_64.so"; \
    fi

COPY --from=builder /app/dist/rivet-engine /usr/bin/rivet-engine

ENTRYPOINT ["/usr/bin/rivet-engine"]
CMD ["start"]


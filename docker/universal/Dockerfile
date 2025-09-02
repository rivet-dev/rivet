# syntax=docker/dockerfile:1.10.0

# MARK: Builder
# TODO(RVT-4168): Compile libfdb from scratch for ARM
FROM --platform=linux/amd64 rust:1.88.0-bookworm AS builder

# Docker automatically provides TARGETARCH
ARG TARGETARCH

ARG BUILD_FRONTEND=false

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
    protobuf-compiler \
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

ENV RIVET_SKIP_BUILD_TS_SDK=1

# Pull via Git CLI to improve reliability in CI
ENV CARGO_NET_GIT_FETCH_WITH_CLI=true

WORKDIR /app

COPY . .

# Build frontend
RUN if [ "$BUILD_FRONTEND" = "true" ]; then \
        (cd sdks/typescript/api-full && pnpm install && pnpm run build) && \
        (cd frontend && pnpm install && pnpm run build); \
    fi

# Build and copy all binaries from target directory into an empty image (it is not
# included in the output because of cache mount)
RUN \
    --mount=type=secret,id=netrc,target=/root/.netrc,mode=0600 \
    --mount=type=cache,target=/usr/local/cargo/git,id=univseral-cargo-git \
    --mount=type=cache,target=/usr/local/cargo/registry,id=univseral-cargo-registry \
    --mount=type=cache,target=/app/target,id=univseral-target \
    --mount=type=cache,target=/root/.cache,id=universal-user-cache \
    RUSTFLAGS="--cfg tokio_unstable" cargo build --bin rivet-engine && \
    # cargo install --locked tokio-console && \
    mkdir /app/dist/ && \
    cp target/debug/rivet-engine /app/dist/

# MARK: Engine (full, base)
FROM debian:12.9-slim AS engine-full-base

# Docker automatically provides TARGETARCH
ARG TARGETARCH

ENV DEBIAN_FRONTEND=noninteractive
# - Install curl for health checks
# - Install go-migrate for running migrations
# - Install database clients to be able to run `rivet db shell ...` (Redis, Postgres, ClickHouse)
RUN apt-get update -y && \
    apt-get install -y \
    ca-certificates \
    openssl \
    curl \
    # redis-tools \
    # postgresql-client \
    gpg \
    dirmngr && \
    # curl -fsSL 'https://packages.clickhouse.com/rpm/lts/repodata/repomd.xml.key' | gpg --dearmor -o /usr/share/keyrings/clickhouse-keyring.gpg && \
    # echo "deb [signed-by=/usr/share/keyrings/clickhouse-keyring.gpg] https://packages.clickhouse.com/deb stable main" | tee /etc/apt/sources.list.d/clickhouse.list && \
    # apt-get update -y && \
    # apt-get install -y clickhouse-client && \
    # (curl -L https://github.com/golang-migrate/migrate/releases/download/v4.18.1/migrate.linux-amd64.tar.gz | tar xvz) && \
    # mv migrate /usr/local/bin/migrate && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/* && \
    if [ "$TARGETARCH" = "arm64" ]; then \
        curl -Lf -o /lib/libfdb_c.so "https://github.com/apple/foundationdb/releases/download/7.3.68/libfdb_c.aarch64.so"; \
    else \
        curl -Lf -o /lib/libfdb_c.so "https://github.com/apple/foundationdb/releases/download/7.3.68/libfdb_c.x86_64.so"; \
    fi

# MARK: Engine (Full)
FROM engine-full-base AS engine-full

LABEL org.opencontainers.image.source https://github.com/rivet-gg/rivet

COPY --from=builder /app/dist/rivet-engine /usr/bin/rivet-engine

CMD ["/usr/bin/rivet-engine", "start"]

# MARK: Engine (Slim)
FROM debian:12.9-slim AS engine-slim

LABEL org.opencontainers.image.source https://github.com/rivet-gg/rivet

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

CMD ["/usr/bin/rivet-engine", "start"]

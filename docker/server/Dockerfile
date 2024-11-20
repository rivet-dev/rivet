# syntax=docker/dockerfile:1.2

# MARK: Builder
FROM rust:1.82.0-slim AS build

RUN apt-get update && \
    DEBIAN_FRONTEND=noninteractive apt-get install -y \
    protobuf-compiler \
    pkg-config \
    libssl-dev \
    g++ \
    git \
    libpq-dev \
    wget \
    ca-certificates \
    openssl \
    curl \
    gpg

WORKDIR /app

COPY . .

# Build and copy all binaries from target directory into an empty image (it is not
# included in the output because of cache mount)
RUN \
	--mount=type=secret,id=netrc,target=/root/.netrc,mode=0600 \
	--mount=type=cache,target=/usr/local/cargo/git,id=server-cargo-git \
	--mount=type=cache,target=/usr/local/cargo/registry,id=server-cargo-registry \
	--mount=type=cache,target=/app/target,id=server-target \
	RUSTFLAGS="--cfg tokio_unstable" cargo build -vv --bin rivet-server --release && \
	mv target/release/rivet-server /usr/bin/rivet-server && \
	mkdir /etc/rivet-server

# MARK: Full
FROM debian:12.1-slim AS full

# - Install curl for health checks
# - Install go-migrate for running migrations
# - Install database clients to be able to run `rivet db shell ...` (Redis, Postgres, ClickHouse)
RUN apt-get update && \
    DEBIAN_FRONTEND=noninteractive apt-get install -y \
    ca-certificates \
    openssl \
    curl \
    redis-tools  \
    postgresql-client \
	gpg && \
    curl -fsSL 'https://packages.clickhouse.com/rpm/lts/repodata/repomd.xml.key' | gpg --dearmor -o /usr/share/keyrings/clickhouse-keyring.gpg && \
    echo "deb [signed-by=/usr/share/keyrings/clickhouse-keyring.gpg] https://packages.clickhouse.com/deb stable main" | tee /etc/apt/sources.list.d/clickhouse.list && \
    apt-get update && \
    DEBIAN_FRONTEND=noninteractive apt-get install -y clickhouse-client && \
    (curl -L https://github.com/golang-migrate/migrate/releases/download/v4.18.1/migrate.linux-amd64.tar.gz | tar xvz) && \
    mv migrate /usr/local/bin/migrate && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

COPY --from=build /usr/bin/rivet-server /usr/bin/rivet-server

CMD ["/usr/bin/rivet-server"]

# MARK: Slim
FROM debian:12.1-slim AS slim

RUN DEBIAN_FRONTEND=noninteractive apt-get update -y && \
	apt-get install -y --no-install-recommends ca-certificates openssl && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

COPY --from=build /usr/bin/rivet-server /usr/bin/rivet-server

CMD ["/usr/bin/rivet-server"]

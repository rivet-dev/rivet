# syntax=docker/dockerfile:1.10.0

# MARK: Builder
# TODO(RVT-4168): Compile libfdb from scratch for ARM
FROM --platform=linux/amd64 rust:1.85.0-bookworm AS builder

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
    wget && \
    curl -fsSL https://deb.nodesource.com/setup_22.x | bash - && \
    apt-get install -y nodejs && \
    corepack enable && \
    curl -Lf -o /lib/libfdb_c.so "https://github.com/apple/foundationdb/releases/download/7.1.60/libfdb_c.x86_64.so"

# Disable interactive prompt
ENV COREPACK_ENABLE_DOWNLOAD_PROMPT=0

# Pull via Git CLI to improve reliability in CI
ENV CARGO_NET_GIT_FETCH_WITH_CLI=true

WORKDIR /app

COPY . .

# Build and copy all binaries from target directory into an empty image (it is not
# included in the output because of cache mount)
RUN \
    --mount=type=secret,id=netrc,target=/root/.netrc,mode=0600 \
    --mount=type=secret,id=fontawesome_package_token,env=FONTAWESOME_PACKAGE_TOKEN \
    --mount=type=cache,target=/usr/local/cargo/git,id=univseral-cargo-git \
    --mount=type=cache,target=/usr/local/cargo/registry,id=univseral-cargo-registry \
    --mount=type=cache,target=/app/target,id=univseral-target \
    --mount=type=cache,target=/app/node_modules,id=universal-node-modules \
    --mount=type=cache,target=/app/.turbo,id=universal-turbo \
    --mount=type=cache,target=/root/.cache,id=universal-user-cache \
    --mount=type=cache,target=/root/.npm,id=universal-user-npm \
    --mount=type=cache,target=/root/.yarn,id=universal-user-yarn \
    RUSTFLAGS="--cfg tokio_unstable" RIVET_BUILD_HUB=0 cargo build --bin rivet-server --bin rivet-edge-server --bin rivet-guard --bin rivet-client --bin rivet-isolate-v8-runner --bin rivet-container-runner && \
    # cargo install --locked tokio-console && \
    mkdir /app/dist/ && \
    cp target/debug/rivet-server target/debug/rivet-edge-server target/debug/rivet-guard target/debug/rivet-client target/debug/rivet-isolate-v8-runner target/debug/rivet-container-runner /app/dist/

# MARK: Server (full, base)
FROM debian:12.9-slim AS server-full-base

ENV DEBIAN_FRONTEND=noninteractive
# - Install curl for health checks
# - Install go-migrate for running migrations
# - Install database clients to be able to run `rivet db shell ...` (Redis, Postgres, ClickHouse)
RUN apt-get update -y && \
    apt-get install -y \
    ca-certificates \
    openssl \
    curl \
    redis-tools  \
    postgresql-client \
    gpg \
    dirmngr && \
    curl -fsSL 'https://packages.clickhouse.com/rpm/lts/repodata/repomd.xml.key' | gpg --dearmor -o /usr/share/keyrings/clickhouse-keyring.gpg && \
    echo "deb [signed-by=/usr/share/keyrings/clickhouse-keyring.gpg] https://packages.clickhouse.com/deb stable main" | tee /etc/apt/sources.list.d/clickhouse.list && \
    apt-get update -y && \
    apt-get install -y clickhouse-client && \
    (curl -L https://github.com/golang-migrate/migrate/releases/download/v4.18.1/migrate.linux-amd64.tar.gz | tar xvz) && \
    mv migrate /usr/local/bin/migrate && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/* && \
    curl -Lf -o /lib/libfdb_c.so "https://github.com/apple/foundationdb/releases/download/7.1.60/libfdb_c.x86_64.so"

# MARK: Server (Full)
FROM server-full-base AS server-full

COPY --from=builder /app/dist/rivet-server /usr/bin/rivet-server

CMD ["/usr/bin/rivet-server"]

# MARK: Server (Slim)
FROM debian:12.9-slim AS server-slim

ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update -y && \
    apt-get install -y ca-certificates openssl curl && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/* && \
    curl -Lf -o /lib/libfdb_c.so "https://github.com/apple/foundationdb/releases/download/7.1.60/libfdb_c.x86_64.so"

COPY --from=builder /app/dist/rivet-server /usr/bin/rivet-server

CMD ["/usr/bin/rivet-server"]

# MARK: Edge server
FROM server-full-base AS edge-server

COPY --from=builder /app/dist/rivet-edge-server /usr/bin

CMD ["/usr/bin/rivet-edge-server"]

# MARK: Rivet Guard
FROM server-full-base AS rivet-guard

COPY --from=builder /app/dist/rivet-guard /usr/bin/rivet-guard

CMD ["/usr/bin/rivet-guard"]

# MARK: Runner base
FROM --platform=linux/amd64 debian:12.9-slim AS base-runner
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update -y && \
    apt-get install -y ca-certificates openssl curl tini curl && \
    curl -Lf -o /lib/libfdb_c.so "https://github.com/apple/foundationdb/releases/download/7.1.60/libfdb_c.x86_64.so"

# MARK: Runner (Full)
FROM --platform=linux/amd64 base-runner AS client-full
ARG CNI_PLUGINS_VERSION=1.3.0
RUN apt-get install -y skopeo iproute2 runc && \
	echo "Downloading lz4" && \
	curl -L https://releases.rivet.gg/tools/lz4/1.10.0/debian11-amd64/lz4 -o /usr/local/bin/lz4 && \
	chmod +x /usr/local/bin/lz4 && \
	echo "Downloading umoci" && \
	curl -Lf -o /usr/bin/umoci "https://github.com/opencontainers/umoci/releases/download/v0.4.7/umoci.amd64" && \
	chmod +x /usr/bin/umoci && \
	echo "Downloading cnitool" && \
	curl -Lf -o /usr/bin/cnitool "https://github.com/rivet-gg/cni/releases/download/v1.1.2-build3/cnitool" && \
	chmod +x /usr/bin/cnitool && \
	echo "Downloading cni-plugins" && \
	mkdir -p /opt/cni/bin /opt/cni/config && \
	curl -L https://github.com/containernetworking/plugins/releases/download/v${CNI_PLUGINS_VERSION}/cni-plugins-linux-amd64-v${CNI_PLUGINS_VERSION}.tgz | \
		tar -xz -C /opt/cni/bin
COPY --from=builder /app/dist/rivet-client /app/dist/rivet-container-runner /usr/local/bin/
ENTRYPOINT ["/usr/bin/tini", "--", "rivet-client"]

# MARK: Monlith
FROM --platform=linux/amd64 debian:12.9-slim AS monolith
ENV DEBIAN_FRONTEND=noninteractive

ARG TRAEFIK_VERSION=3.2.1
ARG COCKROACHDB_VERSION=24.2.3
ARG NATS_VERSION=2.9.2
ARG SEAWEEDFS_VERSION=3.79
# See docs-internal/infrastructure/fdb/AVX.md
ARG FDB_VERSION=7.1.60
ARG VECTOR_VERSION=0.42.0
ARG S6_OVERLAY_VERSION=3.2.0.2

# === Install ===
COPY ./docker/monolith/build-scripts /tmp/build-scripts
# Remove the scripts after running since this pulls dependencies for Deno.
RUN /tmp/build-scripts/install.sh && rm -r /tmp/build-scripts

# === Copy Configs ===
COPY ./docker/monolith/run-scripts/ /root/run-scripts

COPY ./docker/monolith/clickhouse/config.xml /etc/clickhouse-server/config.d/config.xml
COPY ./docker/monolith/clickhouse/users.xml /etc/clickhouse-server/users.d/users.xml

COPY ./docker/monolith/seaweedfs /etc/seaweedfs

COPY ./docker/monolith/foundationdb/etc /etc/foundationdb
COPY --chmod=0755 ./docker/monolith/foundationdb/entrypoint.sh /usr/local/bin/foundationdb-entrypoint.sh

COPY ./docker/monolith/vector-client /etc/vector-client
COPY ./docker/monolith/vector-server /etc/vector-server

COPY ./docker/monolith/rivet-server /etc/rivet-server

COPY ./docker/monolith/rivet-guard /etc/rivet-guard

COPY ./docker/monolith/rivet-client /etc/rivet-client

# === Copy Build Artifacts === 
COPY --from=builder /app/dist/rivet-server /usr/local/bin/
COPY --from=builder /app/dist/rivet-client /app/dist/rivet-isolate-v8-runner /app/dist/rivet-container-runner /usr/local/bin/

VOLUME ["/data"]

EXPOSE 8080 8081 8082 7080 7443 7500-7599 7600-7699

ENV S6_LOGGING=1
ENV S6_VERBOSITY=1
ENV S6_KILL_GRACETIME=100
ENV S6_KILL_FINISH_MAXTIME=100

ENTRYPOINT ["/init"]


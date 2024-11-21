# syntax=docker/dockerfile:1.2

# MARK: Rivet Server Builder
FROM rust:1.82.0-slim AS server-builder

RUN apt-get update && \
    DEBIAN_FRONTEND=noninteractive apt-get install -y \
    protobuf-compiler \
    pkg-config \
    libssl-dev \
    g++ \
    git \
    libpq-dev \
    ca-certificates \
    openssl \
    curl \
    gpg

WORKDIR /app
COPY . .

# TODO(RVT-4152): Add back release mode, currently hangs indefinitely
RUN \
	--mount=type=secret,id=netrc,target=/root/.netrc,mode=0600 \
    --mount=type=cache,target=/usr/local/cargo/git,id=monolith-server-cargo-git \
    --mount=type=cache,target=/usr/local/cargo/registry,id=monolith-server-cargo-registry \
    --mount=type=cache,target=/app/target,id=monolith-server-target \
    RUSTFLAGS="--cfg tokio_unstable" cargo build -v --bin rivet-server && \
    mv target/debug/rivet-server /usr/bin/rivet-server

# MARK: Rivet Client Builder
FROM rust:1.82.0-bullseye AS client-builder

RUN apt-get update && apt-get install --yes protobuf-compiler pkg-config libssl-dev g++ git libpq-dev  && \
    ln -s /bin/g++ /bin/musl-g++ && \
    ln -s /bin/gcc-ar /bin/musl-ar

WORKDIR /app
COPY . .
RUN \
	--mount=type=secret,id=netrc,target=/root/.netrc,mode=0600 \
    --mount=type=cache,target=/usr/local/cargo/git,id=monolith-client-cargo-git \
    --mount=type=cache,target=/usr/local/cargo/registry,id=monolith-client-cargo-registry \
    --mount=type=cache,target=/app/packages/infra/client/target,id=monolith-client-target \
    cd packages/infra/client && \
    RUSTFLAGS="--cfg tokio_unstable" cargo build -v --release --bin rivet-client --bin rivet-isolate-v8-runner --bin rivet-container-runner && \
    mkdir -p /app/dist && \
    mv target/release/rivet-client target/release/rivet-isolate-v8-runner target/release/rivet-container-runner /app/dist/

# MARK: Runner
#
# Requires OpenSSL 1.1, so we pin this to Debian 11 (which uses OpenSSL 1.1)
# instead of 12 (which uses OpenSSL 3).
FROM debian:12-slim AS runner

ARG COCKROACHDB_VERSION=24.2.3
ARG NATS_VERSION=2.9.2
ARG SEAWEEDFS_VERSION=3.79
ARG VECTOR_VERSION=0.42.0
ARG S6_OVERLAY_VERSION=3.2.0.2

# === Install ===
COPY ./docker/monolith/scripts /tmp/scripts
RUN /tmp/scripts/install.sh

# === Copy Configs ===
COPY ./docker/monolith/clickhouse/config.xml /etc/clickhouse-server/config.d/config.xml
COPY ./docker/monolith/clickhouse/users.xml /etc/clickhouse-server/users.d/users.xml

COPY ./docker/monolith/seaweedfs /etc/seaweedfs

COPY ./docker/monolith/vector-client /etc/vector-client
COPY ./docker/monolith/vector-server /etc/vector-server

COPY ./docker/monolith/rivet-server /etc/rivet-server

COPY ./docker/monolith/rivet-client /etc/rivet-client

# === Copy Build Artifacts === 
COPY --from=server-builder /usr/bin/rivet-server /usr/local/bin/
COPY --from=client-builder /app/dist/rivet-client /app/dist/rivet-isolate-v8-runner /app/dist/rivet-container-runner /usr/local/bin/

VOLUME ["/data"]

EXPOSE 8080 8081 8082 8080 20000-20100

ENV S6_LOGGING=1
ENV S6_VERBOSITY=1
ENV S6_KILL_GRACETIME=100
ENV S6_KILL_FINISH_MAXTIME=100

ENTRYPOINT ["/init"]


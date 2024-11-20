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

RUN \
	--mount=type=secret,id=netrc,target=/root/.netrc,mode=0600 \
    --mount=type=cache,target=/usr/local/cargo/git,id=monolith-server-cargo-git \
    --mount=type=cache,target=/usr/local/cargo/registry,id=monolith-server-cargo-registry \
    --mount=type=cache,target=/app/target,id=monolith-server-target \
    RUSTFLAGS="--cfg tokio_unstable" cargo build -vv --release --bin rivet-server && \
    mv target/release/rivet-server /usr/bin/rivet-server

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
    RUSTFLAGS="--cfg tokio_unstable" cargo build -vv --release --bin rivet-client --bin rivet-isolate-v8-runner --bin rivet-container-runner && \
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

RUN uname -m | sed 's/aarch64/arm64/' | sed 's/x86_64/amd64/' > /tmp/arch

# Install required packages
RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y \
    ca-certificates \
    openssl \
    curl \
    postgresql-client \
    gpg \
    xz-utils \
	unzip \
    apt-transport-https \
    dirmngr \
	netcat-openbsd && \
    (curl -L https://github.com/golang-migrate/migrate/releases/download/v4.18.1/migrate.linux-$(cat /tmp/arch).tar.gz | tar xvz) && \
    mv migrate /usr/local/bin/migrate && \
    curl -fsSL https://deno.land/x/install/install.sh | sh && \
    ln -s /root/.deno/bin/deno /usr/local/bin/deno

# === CockroachDB ===
RUN useradd -m -s /bin/bash cockroachdb && \
    curl -sSLf https://binaries.cockroachdb.com/cockroach-v${COCKROACHDB_VERSION}.linux-$(cat /tmp/arch).tgz | tar xz && \
    cp -i cockroach-v${COCKROACHDB_VERSION}.linux-$(cat /tmp/arch)/cockroach /usr/local/bin/ && \
    rm -rf cockroach-v${COCKROACHDB_VERSION}.linux-$(cat /tmp/arch)

# === Redis ===
# TODO(RVT-4084): Switch to Valkey when Debian 13 released or ocmpile from source
RUN useradd -m -s /bin/bash redis && \
    apt install -y redis-server redis-tools

# === ClickHouse ===
RUN useradd -m -s /bin/bash clickhouse && \
    curl -fsSL 'https://packages.clickhouse.com/rpm/lts/repodata/repomd.xml.key' | gpg --dearmor -o /usr/share/keyrings/clickhouse-keyring.gpg && \
    echo "deb [signed-by=/usr/share/keyrings/clickhouse-keyring.gpg] https://packages.clickhouse.com/deb stable main" | tee /etc/apt/sources.list.d/clickhouse.list && \
    apt-get update && \
    DEBIAN_FRONTEND=noninteractive apt-get install -y clickhouse-client clickhouse-server
COPY ./docker/monolith/clickhouse/config.xml /etc/clickhouse-server/config.d/config.xml
COPY ./docker/monolith/clickhouse/users.xml /etc/clickhouse-server/users.d/users.xml

# === NATS ===
RUN useradd -m -s /bin/bash nats && \
    curl -sSLf https://github.com/nats-io/nats-server/releases/download/v${NATS_VERSION}/nats-server-v${NATS_VERSION}-linux-$(cat /tmp/arch).tar.gz | \
    tar xz -C /usr/local/bin/ --strip-components=1 nats-server-v${NATS_VERSION}-linux-$(cat /tmp/arch)/nats-server

# === SeaweedFS ===
RUN useradd -m -s /bin/bash seaweedfs && \
    curl -sSLf https://github.com/seaweedfs/seaweedfs/releases/download/${SEAWEEDFS_VERSION}/linux_$(cat /tmp/arch).tar.gz | tar xz -C /usr/local/bin/
COPY ./docker/monolith/seaweedfs /etc/seaweedfs

# === Vector ===
RUN useradd -m -s /bin/bash vector-client && \
	useradd -m -s /bin/bash vector-server && \
    curl -sSLf https://packages.timber.io/vector/${VECTOR_VERSION}/vector_${VECTOR_VERSION}-1_$(cat /tmp/arch).deb -o /tmp/vector.deb && \
    dpkg -i /tmp/vector.deb && \
    rm /tmp/vector.deb
COPY ./docker/monolith/vector-client /etc/vector-client
COPY ./docker/monolith/vector-server /etc/vector-server

# === S6 Overlay ===
RUN curl -sSLf https://github.com/just-containers/s6-overlay/releases/download/v${S6_OVERLAY_VERSION}/s6-overlay-noarch.tar.xz -o /tmp/s6-overlay-noarch.tar.xz && \
    tar -C / -Jxpf /tmp/s6-overlay-noarch.tar.xz && \
    curl -sSLf https://github.com/just-containers/s6-overlay/releases/download/v${S6_OVERLAY_VERSION}/s6-overlay-$(uname -m).tar.xz -o /tmp/s6-overlay-$(uname -m).tar.xz && \
    tar -C / -Jxpf /tmp/s6-overlay-$(uname -m).tar.xz

# Setup S6
COPY ./docker/monolith/scripts /tmp/scripts
RUN deno run --allow-read --allow-write /tmp/scripts/setup_s6.ts

# === Rivet Server ===
RUN useradd -m -s /bin/bash rivet-server
COPY ./docker/monolith/rivet-server /etc/rivet-server

# === Rivet Client ===
#
# Rivet Client runs as root.
COPY ./docker/monolith/rivet-client /etc/rivet-client

# === Copy Build Artifacts === 
COPY --from=server-builder /usr/bin/rivet-server /usr/local/bin/
COPY --from=client-builder /app/dist/rivet-client /usr/local/bin/
COPY --from=client-builder /app/dist/rivet-isolate-v8-runner /usr/local/bin/
COPY --from=client-builder /app/dist/rivet-container-runner /usr/local/bin/

# Expose ports
VOLUME ["/data"]
EXPOSE 8080 8081 8082 8080 20000-20100

# Prevents logging
ENV S6_LOGGING=1
ENV S6_VERBOSITY=1

# Speed up shutdown
ENV S6_KILL_GRACETIME=100
ENV S6_KILL_FINISH_MAXTIME=100

ENTRYPOINT ["/init"]


# MARK: Builder Isolate Runner
# This version is required for GLIBC 2.31 (used by edge servers on Linode)
FROM rust:1.82.0-bullseye AS builder-isolate-runner

WORKDIR /app
COPY packages/infra/pegboard/runner-protocol/ runner-protocol/
COPY packages/infra/pegboard/v8-isolate-runner/Cargo.toml v8-isolate-runner/Cargo.toml
COPY packages/infra/pegboard/v8-isolate-runner/src/ v8-isolate-runner/src/
RUN \
	--mount=type=cache,target=/root/.cargo/git \
	--mount=type=cache,target=/root/.cargo/registry \
	--mount=type=cache,target=/app/v8-isolate-runner/target \
	cd v8-isolate-runner && \
	cargo build --target x86_64-unknown-linux-gnu && \
	mkdir -p /app/dist && \
	mv /app/v8-isolate-runner/target/x86_64-unknown-linux-gnu/debug/v8-isolate-runner /app/dist/v8-isolate-runner

# MARK: Builder Manager
FROM clux/muslrust:1.81.0-stable AS builder-manager

RUN ln -s /bin/g++ /bin/musl-g++ && \
	ln -s /bin/gcc-ar /bin/musl-ar

WORKDIR /app
COPY . .
RUN \
	--mount=type=cache,target=/root/.cargo/git \
	--mount=type=cache,target=/root/.cargo/registry \
	--mount=type=cache,target=/app/packages/infra/pegboard/target \
	cd packages/infra/pegboard/manager && \
	RUSTFLAGS="--cfg tokio_unstable" cargo build --package pegboard-manager --bin pegboard-manager && \
	mkdir -p /app/dist && \
	mv /app/packages/infra/pegboard/target/x86_64-unknown-linux-musl/debug/pegboard-manager /app/dist/pegboard-manager

# MARK: Runner
#
# This container runs both the manager and the isolate runner alongside each
# other. In production, these processes run on the same machine using systemd.
#
# We don't use separate containers in order to mimic production's behavior.
FROM debian:12.4-slim
RUN apt-get update && apt-get install -y runit
COPY --from=builder-manager /app/dist/pegboard-manager /usr/bin/
COPY --from=builder-isolate-runner /app/dist/v8-isolate-runner /usr/bin/
RUN mkdir -p /etc/service/pegboard-manager /etc/service/v8-isolate-runner
COPY <<EOF /etc/service/pegboard-manager/run
#!/bin/bash
exec /app/dist/pegboard-manager
EOF
COPY <<EOF /etc/service/v8-isolate-runner/run
#!/bin/bash
exec /app/dist/v8-isolate-runner
EOF
RUN chmod +x /etc/service/*/run
CMD ["runsvdir", "/etc/service"]

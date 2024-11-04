# MARK: Builder
FROM rust:1.82.0-bullseye AS builder

RUN apt-get update && apt-get install --yes protobuf-compiler pkg-config libssl-dev g++ git libpq-dev wget && \
	ln -s /bin/g++ /bin/musl-g++ && \
	ln -s /bin/gcc-ar /bin/musl-ar

WORKDIR /app
COPY . .
RUN \
	--mount=type=cache,target=/usr/local/cargo/git \
	--mount=type=cache,target=/usr/local/cargo/registry \
	--mount=type=cache,target=/app/packages/infra/pegboard/target \
	cd packages/infra/pegboard && \
	RUSTFLAGS="--cfg tokio_unstable" cargo build --bin pegboard-manager --bin pegboard-isolate-runner-v8 && \
	mkdir -p /app/dist && \
	mv target/debug/pegboard-manager target/debug/pegboard-isolate-runner-v8 /app/dist/

# MARK: Runner
#
# Requires OpenSSL 1.1, so we pin this to Debian 11 instead of 12 (which uses OpenSSL 3).
FROM debian:11-slim
RUN DEBIAN_FRONTEND=noninteractive apt-get update -y && apt-get install -y --no-install-recommends ca-certificates openssl
COPY --from=builder /app/dist/pegboard-manager /app/dist/pegboard-isolate-runner-v8 /usr/local/bin/
ENTRYPOINT ["pegboard-manager", "-c", "/etc/pegboard/config.json"]


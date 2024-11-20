# MARK: Builder
FROM rust:1.82.0-bullseye AS builder

RUN apt-get update && apt-get install --yes protobuf-compiler pkg-config libssl-dev g++ git libpq-dev wget && \
	ln -s /bin/g++ /bin/musl-g++ && \
	ln -s /bin/gcc-ar /bin/musl-ar

WORKDIR /app
COPY . .
RUN \
	--mount=type=secret,id=netrc,target=/root/.netrc,mode=0600 \
	--mount=type=cache,target=/usr/local/cargo/git,id=client-cargo-git \
	--mount=type=cache,target=/usr/local/cargo/registry,id=client-cargo-registry \
	--mount=type=cache,target=/app/packages/infra/client/target,id=client-target \
	cd packages/infra/client && \
	RUSTFLAGS="--cfg tokio_unstable" cargo build -vv --release --bin rivet-client --bin rivet-isolate-v8-runner --bin rivet-container-runner && \
	mkdir -p /app/dist && \
	mv target/release/rivet-client target/release/rivet-isolate-v8-runner target/release/rivet-container-runner /app/dist/

# MARK: Runner (Full)
FROM debian:12-slim AS full-runner
RUN DEBIAN_FRONTEND=noninteractive apt-get update -y && apt-get install -y --no-install-recommends ca-certificates
COPY --from=builder /app/dist/rivet-client /app/dist/rivet-isolate-v8-runner /app/dist/rivet-container-runner /usr/local/bin/
ENTRYPOINT ["rivet-client"]

# MARK: Runner (Isolate V8)
FROM debian:12-slim AS isolate-v8-runner
RUN DEBIAN_FRONTEND=noninteractive apt-get update -y && apt-get install -y --no-install-recommends ca-certificates
COPY --from=builder /app/dist/rivet-isolate-v8-runner /usr/local/bin/
ENTRYPOINT ["rivet-client"]

# MARK: Runner (Container)
FROM debian:12-slim AS container-runner
RUN DEBIAN_FRONTEND=noninteractive apt-get update -y && apt-get install -y --no-install-recommends ca-certificates
COPY --from=builder /app/dist/rivet-container-runner /usr/local/bin/
ENTRYPOINT ["rivet-client"]

# MARK: Builder
# TODO(RVT-4168): Copmile libfdb from scratch for ARM
FROM --platform=linux/amd64 rust:1.82.0-bullseye AS builder

RUN apt-get update && apt-get install --yes libclang-dev protobuf-compiler pkg-config libssl-dev g++ git wget curl && \
	curl -Lf -o /lib/libfdb_c.so "https://github.com/apple/foundationdb/releases/download/7.1.60/libfdb_c.x86_64.so"

WORKDIR /app
COPY . .
RUN \
	--mount=type=secret,id=netrc,target=/root/.netrc,mode=0600 \
	--mount=type=cache,target=/usr/local/cargo/git,id=dev-full-client-cargo-git \
	--mount=type=cache,target=/usr/local/cargo/registry,id=dev-full-client-cargo-registry \
	--mount=type=cache,target=/app/packages/infra/client/target,id=dev-full-client-target \
	cd packages/infra/client && \
	RUSTFLAGS="--cfg tokio_unstable" cargo build --bin rivet-client --bin rivet-isolate-v8-runner --bin rivet-container-runner && \
	mkdir -p /app/dist && \
	mv target/debug/rivet-client target/debug/rivet-isolate-v8-runner target/debug/rivet-container-runner /app/dist/

# MARK: Runner
FROM --platform=linux/amd64 debian:12-slim
# The FDB version should match `cluster::workflows::server::install::install_scripts::components::fdb::FDB_VERSION`
RUN DEBIAN_FRONTEND=noninteractive apt-get update -y && \
	apt-get install -y --no-install-recommends ca-certificates curl && \
	curl -Lf -o /lib/libfdb_c.so "https://github.com/apple/foundationdb/releases/download/7.1.60/libfdb_c.x86_64.so" && \
	curl -Lf -o /usr/local/bin/fdbcli "https://github.com/apple/foundationdb/releases/download/7.1.60/fdbcli.x86_64" && \
	chmod +x /usr/local/bin/fdbcli
COPY --from=builder /app/dist/rivet-client /app/dist/rivet-isolate-v8-runner /app/dist/rivet-container-runner /usr/local/bin/
ENTRYPOINT ["rivet-client"]
CMD ["-c", "/etc/rivet-client/config.json"]

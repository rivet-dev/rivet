# This version is required for GLIBC 2.31 (used by edge servers on Linode)
FROM rust:1.82.0-bullseye AS rust

WORKDIR /app
COPY . .

# Installs shared libs
#
# The FDB version should match `cluster::workflows::server::install::install_scripts::components::fdb::FDB_VERSION`
RUN apt-get update && apt-get install -y libclang-dev protobuf-compiler && \
	curl -Lf -o /lib/libfdb_c.so "https://github.com/apple/foundationdb/releases/download/7.1.60/libfdb_c.x86_64.so"

RUN \
	--mount=type=cache,target=/root/.cargo/git \
	--mount=type=cache,target=/root/.cargo/registry \
	--mount=type=cache,target=/app/packages/infra/client/target \
	cd packages/infra/client/isolate-v8-runner && \
	RUSTFLAGS="--cfg tokio_unstable" cargo build --release --target x86_64-unknown-linux-gnu --bin rivet-isolate-v8-runner && \
	mkdir -p /app/dist && \
	mv /app/packages/infra/client/target/x86_64-unknown-linux-gnu/release/rivet-isolate-v8-runner /app/dist/rivet-isolate-v8-runner

# Create an empty image and copy binaries into it to minimize the size of the image
FROM scratch
COPY --from=rust /app/dist/ /

# Allows `docker create` to work even though this fails
CMD [""]

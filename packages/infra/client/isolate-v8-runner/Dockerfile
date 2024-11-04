# This version is required for GLIBC 2.31 (used by edge servers on Linode)
FROM rust:1.82.0-bullseye AS rust

WORKDIR /app
COPY packages/infra/client/runner-protocol/ runner-protocol/
COPY packages/infra/client/isolate-v8-runner/Cargo.toml isolate-v8-runner/Cargo.toml
COPY packages/infra/client/isolate-v8-runner/src/ isolate-v8-runner/src/
RUN \
	--mount=type=cache,target=/root/.cargo/git \
	--mount=type=cache,target=/root/.cargo/registry \
	--mount=type=cache,target=/app/isolate-v8-runner/target \
	cd isolate-v8-runner && \
	cargo build --release --target x86_64-unknown-linux-gnu && \
	mkdir -p /app/dist && \
	mv /app/isolate-v8-runner/target/x86_64-unknown-linux-gnu/release/isolate-v8-runner /app/dist/isolate-v8-runner

# Create an empty image and copy binaries into it to minimize the size of the image
FROM scratch
COPY --from=rust /app/dist/ /

# Allows `docker create` to work even though this fails
CMD [""]

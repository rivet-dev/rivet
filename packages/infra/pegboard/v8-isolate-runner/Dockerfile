FROM clux/muslrust:1.80.0-stable AS rust

WORKDIR /app
COPY packages/infra/pegboard/runner-protocol/ runner-protocol/
COPY packages/infra/pegboard/v8-isolate-runner/Cargo.toml v8-isolate-runner/Cargo.toml
COPY packages/infra/pegboard/v8-isolate-runner/src/ v8-isolate-runner/src/
RUN \
	--mount=type=cache,target=/root/.cargo/git \
	--mount=type=cache,target=/root/.cargo/registry \
	--mount=type=cache,target=/app/v8-isolate-runner/target \
	cd v8-isolate-runner && \
	cargo build --release --target x86_64-unknown-linux-gnu && \
	mkdir -p /app/dist && \
	mv /app/v8-isolate-runner/target/x86_64-unknown-linux-gnu/release/v8-isolate-runner /app/dist/v8-isolate-runner

# Create an empty image and copy binaries into it to minimize the size of the image
FROM scratch
COPY --from=rust /app/dist/ /

# Allows `docker create` to work even though this fails
CMD [""]

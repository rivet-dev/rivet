FROM clux/muslrust:1.81.0-stable AS rust

WORKDIR /app
COPY . .
RUN \
	--mount=type=cache,target=/root/.cargo/git \
	--mount=type=cache,target=/root/.cargo/registry \
	--mount=type=cache,target=/app/target \
	RUSTFLAGS="--cfg tokio_unstable" cargo build --release --bin pegboard-echo-server && \
	mkdir -p /app/dist && \
	mv /app/target/x86_64-unknown-linux-musl/release/pegboard-echo-server /app/dist/pegboard-echo-server

# Create non-root user and group
RUN useradd -m -d /home/nonroot -s /bin/sh nonroot

# Create a minimal passwd and group file with the nonroot user info
RUN grep nonroot /etc/passwd > /passwd && \
    grep nonroot /etc/group > /group

# Create an empty image and copy binaries into it to minimize the size of the image
FROM scratch

# Copy passwd and group files
COPY --from=rust /passwd /etc/passwd
COPY --from=rust /group /etc/group

COPY --from=rust /app/dist/ /

# Switch to the non-root user
USER nonroot

CMD ["/pegboard-echo-server"]

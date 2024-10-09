FROM clux/muslrust:1.80.0-stable AS rust

WORKDIR /app
COPY Cargo.toml .
COPY src/ src/
RUN cargo build --release

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

COPY --from=rust /app/target/x86_64-unknown-linux-musl/release/echo-server /echo-server

# Switch to the non-root user
USER nonroot

CMD ["/echo-server"]

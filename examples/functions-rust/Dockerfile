FROM rust:1.77 as builder

WORKDIR /app
COPY Cargo.toml .
COPY src ./src

RUN cargo build --release

FROM debian:bullseye-slim

WORKDIR /app

# Create rivet user and set permissions
RUN groupadd -r rivet && useradd -r -g rivet rivet

# Install necessary dependencies for running the binary
RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/*

# Copy the built binary from the builder stage
COPY --from=builder /app/target/release/functions-rust /app/functions-rust
RUN chown -R rivet:rivet /app

# Switch to non-root user
USER rivet

# Run the binary
CMD ["/app/functions-rust"]
FROM clux/muslrust:1.81.0-stable

WORKDIR /app
COPY Cargo.toml .
COPY src/ src/
RUN cargo build --release


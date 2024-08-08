FROM clux/muslrust:1.80.0-stable

WORKDIR /app
COPY Cargo.toml Cargo.lock .
COPY src/ src/
RUN cargo build --release


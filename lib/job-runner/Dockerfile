FROM clux/muslrust:1.73.0

WORKDIR /app
COPY Cargo.toml Cargo.lock .
COPY src/ src/
RUN cargo build --release


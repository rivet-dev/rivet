FROM clux/muslrust:1.77.2

WORKDIR /app
COPY Cargo.toml Cargo.lock .
COPY src/ src/
RUN cargo build --release


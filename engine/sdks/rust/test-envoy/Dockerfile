FROM rust:1.88-slim AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY engine ./engine

RUN cargo build --release -p rivet-test-envoy

FROM debian:bookworm-slim

RUN apt-get update \
	&& apt-get install -y --no-install-recommends ca-certificates curl \
	&& rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/rivet-test-envoy /usr/local/bin/rivet-test-envoy

EXPOSE 5050

CMD ["rivet-test-envoy"]

FROM clux/muslrust:1.73.0 AS build
RUN cargo new --bin /app
WORKDIR /app
COPY Cargo.toml ./
RUN cargo build --release
RUN rm -r src
COPY ./src ./src
RUN cargo build --release \
    && strip target/x86_64-unknown-linux-musl/release/test-mm-lobby-echo

FROM alpine:latest
COPY --from=build /app/target/x86_64-unknown-linux-musl/release/test-mm-lobby-echo /usr/local/bin/app
RUN chmod +x /usr/local/bin/app
CMD ["/usr/local/bin/app"]


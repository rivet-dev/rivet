FROM clux/muslrust:1.73.0 AS build
WORKDIR /app
COPY . .
RUN cargo build --release \
    && strip target/x86_64-unknown-linux-musl/release/test-mm-lobby-echo

FROM alpine:latest
COPY --from=build /app/target/x86_64-unknown-linux-musl/release/test-mm-lobby-echo /usr/local/bin/app
RUN chmod +x /usr/local/bin/app
CMD ["/usr/local/bin/app"]


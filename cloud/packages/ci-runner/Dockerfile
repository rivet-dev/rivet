FROM alpine:latest AS builder

# FROM gcr.io/kaniko-project/executor:latest
FROM ghcr.io/rivet-gg/executor@sha256:439d4dbb0f3f8c1c6c2195e144d29195b4930b87166a6fae32061b564a6baa9e
COPY --from=builder /bin/sh /bin/sh
COPY --from=builder /lib/ld-musl-x86_64.so.1 /lib/ld-musl-x86_64.so.1

COPY entry.sh ~/entry.sh

ENTRYPOINT ["/bin/sh", "~/entry.sh"]

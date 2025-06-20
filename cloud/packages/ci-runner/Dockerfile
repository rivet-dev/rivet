FROM alpine:latest AS builder

# FROM gcr.io/kaniko-project/executor:latest
FROM ghcr.io/rivet-gg/executor@sha256:439d4dbb0f3f8c1c6c2195e144d29195b4930b87166a6fae32061b564a6baa9e
COPY --from=builder /bin/sh /bin/sh
COPY --from=builder /lib/ld-musl-x86_64.so.1 /lib/ld-musl-x86_64.so.1

# HACK: Use env vars to interpolate args bc Rivet doesn't support passing args
ENTRYPOINT ["/bin/sh", "-c", "/kaniko/executor --context=${CONTEXT_URL} --destination=${DESTINATION} --upload-tar=${OUTPUT_URL} --dockerfile=${DOCKERFILE_PATH} --no-push --single-snapshot --verbosity=info"]

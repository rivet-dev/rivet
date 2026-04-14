# syntax=docker/dockerfile:1.10.0

# MARK: Builder
# TODO(RVT-4168): Compile libfdb from scratch for ARM
FROM ghcr.io/rivet-dev/rivet/engine-base-builder:a36b881 AS builder

# Docker automatically provides TARGETARCH
ARG TARGETARCH

ARG BUILD_FRONTEND=false
ARG CARGO_BUILD_MODE=debug
ARG VITE_APP_API_URL=__SAME__
ARG OVERRIDE_GIT_SHA

WORKDIR /app

COPY . .

# Build frontend. Use --ignore-scripts because the root postinstall runs
# `lefthook install`, which needs a .git directory (excluded by
# .dockerignore). lefthook is a dev-only git hook manager and has no
# place inside the Docker build. SKIP_NAPI_BUILD=1 tells
# @rivetkit/rivetkit-native to skip its napi build — the frontend only
# consumes the TypeScript surface, not the runtime .node binary.
RUN if [ "$BUILD_FRONTEND" = "true" ]; then \
        export NODE_OPTIONS="--max-old-space-size=8192" && \
        export SKIP_NAPI_BUILD=1 && \
        pnpm install --ignore-scripts && \
        if [ -n "$VITE_APP_API_URL" ]; then \
            VITE_APP_API_URL="${VITE_APP_API_URL}" npx turbo build:engine -F @rivetkit/engine-frontend; \
        else \
            npx turbo build:engine -F @rivetkit/engine-frontend; \
        fi; \
    fi

# Build and copy all binaries from target directory into an empty image (it is not
# included in the output because of cache mount)
RUN \
    --mount=type=secret,id=netrc,target=/root/.netrc,mode=0600 \
    --mount=type=cache,target=/usr/local/cargo/git,id=univseral-cargo-git \
    --mount=type=cache,target=/usr/local/cargo/registry,id=univseral-cargo-registry \
    --mount=type=cache,target=/app/target,id=univseral-target \
    --mount=type=cache,target=/root/.cache,id=universal-user-cache \
    if [ "$CARGO_BUILD_MODE" = "release" ]; then \
        RUSTFLAGS="--cfg tokio_unstable" cargo build --bin rivet-engine --release; \
    else \
        RUSTFLAGS="--cfg tokio_unstable" cargo build --bin rivet-engine; \
    fi && \
    # cargo install --locked tokio-console && \
    mkdir /app/dist/ && \
    echo "Copying binary" && \
    cp target/$CARGO_BUILD_MODE/rivet-engine /app/dist/

# MARK: Engine (full, base)
FROM ghcr.io/rivet-dev/rivet/engine-base-runtime-full:a36b881 AS engine-full-base

# MARK: Engine (Full)
FROM engine-full-base AS engine-full

LABEL org.opencontainers.image.source=https://github.com/rivet-dev/rivet

COPY --from=builder /app/dist/rivet-engine /usr/bin/rivet-engine

ENTRYPOINT ["/usr/bin/rivet-engine"]
CMD ["start"]

# MARK: Engine (Slim)
FROM ghcr.io/rivet-dev/rivet/engine-base-runtime-slim:a36b881 AS engine-slim

LABEL org.opencontainers.image.source=https://github.com/rivet-dev/rivet

COPY --from=builder /app/dist/rivet-engine /usr/bin/rivet-engine

ENTRYPOINT ["/usr/bin/rivet-engine"]
CMD ["start"]

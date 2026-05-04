# syntax=docker/dockerfile:1.10.0

# MARK: Builder
# TODO(RVT-4168): Compile libfdb from scratch for ARM
FROM ghcr.io/rivet-dev/rivet/engine-base-builder:a36b881 AS builder

# Docker automatically provides TARGETARCH
ARG TARGETARCH

ARG BUILD_FRONTEND=false
ARG VITE_FEATURE_FLAGS=
ARG CARGO_BUILD_MODE=debug
ARG VITE_APP_API_URL=__SAME__
ARG VITE_APP_TURNSTILE_SITE_KEY=
ARG OVERRIDE_GIT_SHA
ARG RUST_TOOLCHAIN=1.91.1

WORKDIR /app

COPY . .

RUN rustup toolchain install "${RUST_TOOLCHAIN}" --profile minimal && \
    rustup default "${RUST_TOOLCHAIN}"

# Build frontend. Use --ignore-scripts because the root postinstall runs
# `lefthook install`, which needs a .git directory (excluded by
# .dockerignore). lefthook is a dev-only git hook manager and has no
# place inside the Docker build. SKIP_NAPI_BUILD=1 and SKIP_WASM_BUILD=1 tell
# the runtime binding packages to skip native artifact builds. The frontend only
# consumes their TypeScript surfaces, not the runtime binaries.
RUN if [ "$BUILD_FRONTEND" = "true" ]; then \
        export NODE_OPTIONS="--max-old-space-size=8192" && \
        export SKIP_NAPI_BUILD=1 && \
        export SKIP_WASM_BUILD=1 && \
        pnpm install --ignore-scripts && \
        VITE_APP_API_URL="${VITE_APP_API_URL}" VITE_FEATURE_FLAGS="${VITE_FEATURE_FLAGS}" VITE_APP_TURNSTILE_SITE_KEY="${VITE_APP_TURNSTILE_SITE_KEY}" npx turbo build -F @rivetkit/engine-frontend; \
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
        RUSTFLAGS="--cfg tokio_unstable" cargo build -p rivet-engine --bin rivet-engine --release; \
    else \
        RUSTFLAGS="--cfg tokio_unstable" cargo build -p rivet-engine --bin rivet-engine; \
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

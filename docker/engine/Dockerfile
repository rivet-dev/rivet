# syntax=docker/dockerfile:1.10.0

# MARK: Builder
# TODO(RVT-4168): Compile libfdb from scratch for ARM
FROM ghcr.io/rivet-dev/rivet/engine-base-builder:a36b881 AS builder

# Docker automatically provides TARGETARCH
ARG TARGETARCH

ARG CARGO_BUILD_MODE=debug
ARG STRIP_BINARY=false
ARG OVERRIDE_GIT_SHA

WORKDIR /app

COPY . .

# Install the channel pinned in rust-toolchain.toml, copied in above.
RUN RUST_TOOLCHAIN="$(sed -n 's/^channel = "\(.*\)"$/\1/p' rust-toolchain.toml)" && \
    rustup toolchain install "${RUST_TOOLCHAIN}" --profile minimal && \
    rustup default "${RUST_TOOLCHAIN}"

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
    cp target/$CARGO_BUILD_MODE/rivet-engine /app/dist/ && \
    if [ "$STRIP_BINARY" = "true" ]; then strip /app/dist/rivet-engine; fi

# MARK: Engine (full, base)
FROM ghcr.io/rivet-dev/rivet/engine-base-runtime-full:a36b881 AS engine-full-base

# MARK: Engine (Full)
FROM engine-full-base AS engine-full

LABEL org.opencontainers.image.source=https://github.com/rivet-dev/rivet

COPY --from=builder /app/dist/rivet-engine /usr/bin/rivet-engine

ENTRYPOINT ["/usr/bin/rivet-engine"]

# MARK: Engine (Slim)
FROM ghcr.io/rivet-dev/rivet/engine-base-runtime-slim:a36b881 AS engine-slim

LABEL org.opencontainers.image.source=https://github.com/rivet-dev/rivet

COPY --from=builder /app/dist/rivet-engine /usr/bin/rivet-engine

ENTRYPOINT ["/usr/bin/rivet-engine"]

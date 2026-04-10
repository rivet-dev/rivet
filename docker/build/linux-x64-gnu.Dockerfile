# syntax=docker/dockerfile:1.10.0
# Unified build for linux-x64-gnu.
# Builds either rivet-engine or rivetkit-native based on BUILD_TARGET.
#
# Build args:
#   BASE_TAG        - base image tag (set by build-push script)
#   BUILD_TARGET    - "engine" or "rivetkit-native"
#   BUILD_MODE      - "debug" (fast) or "release" (optimized)
#   BUILD_FRONTEND  - "true" or "false" (engine only)
#
# Base image: docker/builder-base/linux-gnu.Dockerfile
# Rebuild base: scripts/docker-builder-base/build-push.sh linux-gnu --push
ARG BASE_TAG=latest
FROM ghcr.io/rivet-dev/rivet/builder-base-linux-gnu:${BASE_TAG}

ARG BUILD_TARGET=engine
ARG BUILD_MODE=release
ARG BUILD_FRONTEND=false
ARG VITE_APP_API_URL=__SAME__

ENV RUSTFLAGS="--cfg tokio_unstable"

WORKDIR /build
COPY . .

# Build frontend if building engine with frontend enabled.
RUN if [ "$BUILD_TARGET" = "engine" ] && [ "$BUILD_FRONTEND" = "true" ]; then \
        export NODE_OPTIONS="--max-old-space-size=8192" && \
        pnpm install && \
        if [ -n "$VITE_APP_API_URL" ]; then \
            VITE_APP_API_URL="${VITE_APP_API_URL}" npx turbo build:engine -F @rivetkit/engine-frontend; \
        else \
            npx turbo build:engine -F @rivetkit/engine-frontend; \
        fi; \
    fi

# Build binary.
RUN --mount=type=cache,id=cargo-registry-linux-x64-gnu,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=cargo-git-linux-x64-gnu,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,id=cargo-target-linux-x64-gnu,target=/build/target,sharing=locked \
    set -e && \
    if [ "$BUILD_MODE" = "release" ]; then \
        CARGO_FLAG="--release"; \
        PROFILE_DIR="release"; \
    else \
        CARGO_FLAG=""; \
        PROFILE_DIR="debug"; \
    fi && \
    mkdir -p /artifacts && \
    if [ "$BUILD_TARGET" = "engine" ]; then \
        cargo build --bin rivet-engine $CARGO_FLAG --target x86_64-unknown-linux-gnu && \
        cp target/x86_64-unknown-linux-gnu/$PROFILE_DIR/rivet-engine /artifacts/rivet-engine-x86_64-unknown-linux-gnu; \
    elif [ "$BUILD_TARGET" = "rivetkit-native" ]; then \
        cd rivetkit-typescript/packages/rivetkit-native && \
        napi build --platform $CARGO_FLAG --target x86_64-unknown-linux-gnu && \
        cp rivetkit-native.linux-x64-gnu.node /artifacts/; \
    else \
        echo "Unknown BUILD_TARGET: $BUILD_TARGET" && exit 1; \
    fi

CMD ["ls", "-la", "/artifacts"]

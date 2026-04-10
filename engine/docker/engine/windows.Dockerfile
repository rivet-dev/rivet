# syntax=docker/dockerfile:1.10.0
# Base image built from: engine/docker/builder-base/windows-mingw.Dockerfile
# Rebuild base: scripts/docker-builder-base/build-push.sh windows-mingw --push
FROM ghcr.io/rivet-dev/rivet/builder-base-windows-mingw:9a730d455

ARG BUILD_FRONTEND=true
ARG VITE_APP_API_URL=__SAME__

ENV RUSTFLAGS="--cfg tokio_unstable"

WORKDIR /build

COPY . .

RUN if [ "$BUILD_FRONTEND" = "true" ]; then \
        export NODE_OPTIONS="--max-old-space-size=8192" && \
        pnpm install && \
        if [ -n "$VITE_APP_API_URL" ]; then \
            VITE_APP_API_URL="${VITE_APP_API_URL}" npx turbo build:engine -F @rivetkit/engine-frontend; \
        else \
            npx turbo build:engine -F @rivetkit/engine-frontend; \
        fi; \
    fi

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/build/target \
    cargo build --bin rivet-engine --release --target x86_64-pc-windows-gnu && \
    mkdir -p /artifacts && \
    cp target/x86_64-pc-windows-gnu/release/rivet-engine.exe /artifacts/rivet-engine-x86_64-pc-windows-gnu.exe

CMD ["ls", "-la", "/artifacts"]

# syntax=docker/dockerfile:1.10.0
# Base image built from: engine/docker/builder-base/linux-gnu.Dockerfile
# Rebuild base: scripts/docker-builder-base/build-push.sh linux-gnu --push
FROM ghcr.io/rivet-dev/rivet/builder-base-linux-gnu:TODO

WORKDIR /build
COPY . .

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/build/target \
    cd rivetkit-typescript/packages/rivetkit-native && \
    napi build --platform --release --target x86_64-unknown-linux-gnu && \
    mkdir -p /artifacts && \
    cp rivetkit-native.linux-x64-gnu.node /artifacts/

CMD ["ls", "-la", "/artifacts"]

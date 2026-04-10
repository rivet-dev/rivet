# syntax=docker/dockerfile:1.10.0
# Base image built from: engine/docker/builder-base/windows-msvc.Dockerfile
# Rebuild base: scripts/docker-builder-base/build-push.sh windows-msvc --push
FROM ghcr.io/rivet-dev/rivet/builder-base-windows-msvc:TODO

WORKDIR /build
COPY . .

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/build/target \
    cd rivetkit-typescript/packages/rivetkit-native && \
    cargo xwin build --release --target x86_64-pc-windows-msvc -p rivetkit-native && \
    mkdir -p /artifacts && \
    cp /build/target/x86_64-pc-windows-msvc/release/rivetkit_native.dll /artifacts/rivetkit-native.win32-x64-msvc.node

CMD ["ls", "-la", "/artifacts"]

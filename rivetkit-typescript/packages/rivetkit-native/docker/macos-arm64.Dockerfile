# syntax=docker/dockerfile:1.10.0
# Base image built from: engine/docker/builder-base/osxcross.Dockerfile
# Rebuild base: scripts/docker-builder-base/build-push.sh osxcross --push
FROM ghcr.io/rivet-dev/rivet/builder-base-osxcross:TODO

# aarch64-specific cross-compilation env
ENV BINDGEN_EXTRA_CLANG_ARGS_aarch64_apple_darwin="--sysroot=/root/osxcross/target/SDK/MacOSX11.3.sdk -isystem /root/osxcross/target/SDK/MacOSX11.3.sdk/usr/include" \
    CFLAGS_aarch64_apple_darwin="-B/root/osxcross/target/bin" \
    CXXFLAGS_aarch64_apple_darwin="-B/root/osxcross/target/bin" \
    CARGO_TARGET_AARCH64_APPLE_DARWIN_LINKER=aarch64-apple-darwin20.4-clang \
    CC_aarch64_apple_darwin=aarch64-apple-darwin20.4-clang \
    CXX_aarch64_apple_darwin=aarch64-apple-darwin20.4-clang++ \
    AR_aarch64_apple_darwin=aarch64-apple-darwin20.4-ar \
    RANLIB_aarch64_apple_darwin=aarch64-apple-darwin20.4-ranlib

RUN mkdir -p /root/.cargo && \
    echo '[target.aarch64-apple-darwin]\nlinker = "aarch64-apple-darwin20.4-clang"\nar = "aarch64-apple-darwin20.4-ar"\n' > /root/.cargo/config.toml

WORKDIR /build
COPY . .

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/build/target \
    cd rivetkit-typescript/packages/rivetkit-native && \
    NAPI_RS_CROSS_COMPILE=1 napi build --platform --release --target aarch64-apple-darwin && \
    mkdir -p /artifacts && \
    cp rivetkit-native.darwin-arm64.node /artifacts/

CMD ["ls", "-la", "/artifacts"]

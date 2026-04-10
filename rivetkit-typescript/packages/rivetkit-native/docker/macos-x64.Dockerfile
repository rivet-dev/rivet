# syntax=docker/dockerfile:1.10.0
# Base image built from: engine/docker/builder-base/osxcross.Dockerfile
# Rebuild base: scripts/docker-builder-base/build-push.sh osxcross --push
FROM ghcr.io/rivet-dev/rivet/builder-base-osxcross:TODO

# x86_64-specific cross-compilation env
ENV BINDGEN_EXTRA_CLANG_ARGS_x86_64_apple_darwin="--sysroot=/root/osxcross/target/SDK/MacOSX11.3.sdk -isystem /root/osxcross/target/SDK/MacOSX11.3.sdk/usr/include" \
    CFLAGS_x86_64_apple_darwin="-B/root/osxcross/target/bin" \
    CXXFLAGS_x86_64_apple_darwin="-B/root/osxcross/target/bin" \
    CARGO_TARGET_X86_64_APPLE_DARWIN_LINKER=x86_64-apple-darwin20.4-clang \
    CC_x86_64_apple_darwin=x86_64-apple-darwin20.4-clang \
    CXX_x86_64_apple_darwin=x86_64-apple-darwin20.4-clang++ \
    AR_x86_64_apple_darwin=x86_64-apple-darwin20.4-ar \
    RANLIB_x86_64_apple_darwin=x86_64-apple-darwin20.4-ranlib

RUN mkdir -p /root/.cargo && \
    echo '[target.x86_64-apple-darwin]\nlinker = "x86_64-apple-darwin20.4-clang"\nar = "x86_64-apple-darwin20.4-ar"\n' > /root/.cargo/config.toml

WORKDIR /build
COPY . .

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/build/target \
    cd rivetkit-typescript/packages/rivetkit-native && \
    NAPI_RS_CROSS_COMPILE=1 napi build --platform --release --target x86_64-apple-darwin && \
    mkdir -p /artifacts && \
    cp rivetkit-native.darwin-x64.node /artifacts/

CMD ["ls", "-la", "/artifacts"]

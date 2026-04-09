# syntax=docker/dockerfile:1.10.0
FROM rust:1.89.0-bookworm AS base

# Install dependencies and osxcross
RUN apt-get update && apt-get install -y \
    git-lfs \
    clang \
    cmake \
    patch \
    libxml2-dev \
    wget \
    xz-utils \
    curl && \
    curl -fsSL https://deb.nodesource.com/setup_22.x | bash - && \
    apt-get install -y nodejs && \
    npm install -g @napi-rs/cli && \
    rm -rf /var/lib/apt/lists/*

# Install osxcross
RUN git config --global --add safe.directory '*' && \
    git clone https://github.com/tpoechtrager/osxcross /root/osxcross && \
    cd /root/osxcross && \
    wget -nc https://github.com/phracker/MacOSX-SDKs/releases/download/11.3/MacOSX11.3.sdk.tar.xz && \
    mv MacOSX11.3.sdk.tar.xz tarballs/ && \
    UNATTENDED=yes OSX_VERSION_MIN=10.7 ./build.sh

ENV PATH="/root/osxcross/target/bin:$PATH"

ENV OSXCROSS_SDK=MacOSX11.3.sdk \
    SDKROOT=/root/osxcross/target/SDK/MacOSX11.3.sdk \
    BINDGEN_EXTRA_CLANG_ARGS_aarch64_apple_darwin="--sysroot=/root/osxcross/target/SDK/MacOSX11.3.sdk -isystem /root/osxcross/target/SDK/MacOSX11.3.sdk/usr/include" \
    CFLAGS_aarch64_apple_darwin="-B/root/osxcross/target/bin" \
    CXXFLAGS_aarch64_apple_darwin="-B/root/osxcross/target/bin" \
    CARGO_TARGET_AARCH64_APPLE_DARWIN_LINKER=aarch64-apple-darwin20.4-clang \
    CC_aarch64_apple_darwin=aarch64-apple-darwin20.4-clang \
    CXX_aarch64_apple_darwin=aarch64-apple-darwin20.4-clang++ \
    AR_aarch64_apple_darwin=aarch64-apple-darwin20.4-ar \
    RANLIB_aarch64_apple_darwin=aarch64-apple-darwin20.4-ranlib \
    MACOSX_DEPLOYMENT_TARGET=10.14 \
    CARGO_INCREMENTAL=0 \
    CARGO_NET_GIT_FETCH_WITH_CLI=true

RUN rustup target add aarch64-apple-darwin && \
    mkdir -p /root/.cargo && \
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

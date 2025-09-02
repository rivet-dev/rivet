# syntax=docker/dockerfile:1.4
FROM rust:1.88.0 AS base

# Install dependencies
RUN apt-get update && apt-get install -y \
    git-lfs \
    protobuf-compiler \
    clang \
    cmake \
    patch \
    libxml2-dev \
    wget \
    xz-utils \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Install osxcross
RUN git config --global --add safe.directory '*' && \
    git clone https://github.com/tpoechtrager/osxcross /root/osxcross && \
    cd /root/osxcross && \
    wget -nc https://github.com/phracker/MacOSX-SDKs/releases/download/11.3/MacOSX11.3.sdk.tar.xz && \
    mv MacOSX11.3.sdk.tar.xz tarballs/ && \
    UNATTENDED=yes OSX_VERSION_MIN=10.7 ./build.sh

# Add osxcross to PATH
ENV PATH="/root/osxcross/target/bin:$PATH"

# Tell Clang/bindgen to use the macOS SDK, and nudge Clang to prefer osxcross binutils.
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
    RUSTFLAGS="--cfg tokio_unstable" \
    CARGO_INCREMENTAL=0 \
    CARGO_NET_GIT_FETCH_WITH_CLI=true

# Set working directory
WORKDIR /build

# Build for ARM64 macOS
FROM base AS aarch64-builder

# Install macOS ARM64 target
RUN rustup target add aarch64-apple-darwin

# Configure Cargo for cross-compilation (ARM64)
RUN mkdir -p /root/.cargo && \
    echo '\
[target.aarch64-apple-darwin]\n\
linker = "aarch64-apple-darwin20.4-clang"\n\
ar = "aarch64-apple-darwin20.4-ar"\n\
' > /root/.cargo/config.toml

# Copy the source code
COPY . .

# Build for ARM64 macOS
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/build/target \
    cargo build --bin rivet-engine --release --target aarch64-apple-darwin && \
    mkdir -p /artifacts && \
    cp target/aarch64-apple-darwin/release/rivet-engine /artifacts/rivet-engine-aarch64-apple-darwin

# Default command to show help
CMD ["ls", "-la", "/artifacts"]

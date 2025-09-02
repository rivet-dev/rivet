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
    BINDGEN_EXTRA_CLANG_ARGS_X86_64_apple_darwin="--sysroot=/root/osxcross/target/SDK/MacOSX11.3.sdk -isystem /root/osxcross/target/SDK/MacOSX11.3.sdk/usr/include" \
    CFLAGS_X86_64_apple_darwin="-B/root/osxcross/target/bin" \
    CXXFLAGS_X86_64_apple_darwin="-B/root/osxcross/target/bin" \
    CARGO_TARGET_X86_64_APPLE_DARWIN_LINKER=x86_64-apple-darwin20.4-clang \
    CC_x86_64_apple_darwin=x86_64-apple-darwin20.4-clang \
    CXX_x86_64_apple_darwin=x86_64-apple-darwin20.4-clang++ \
    AR_X86_64_apple_darwin=x86_64-apple-darwin20.4-ar \
    RANLIB_X86_64_apple_darwin=x86_64-apple-darwin20.4-ranlib \
    MACOSX_DEPLOYMENT_TARGET=10.14 \
    RUSTFLAGS="--cfg tokio_unstable" \
    CARGO_INCREMENTAL=0 \
    CARGO_NET_GIT_FETCH_WITH_CLI=true

# Set working directory
WORKDIR /build

# Build for x86_64 macOS
FROM base AS x86_64-builder

# Install macOS x86_64 target
RUN rustup target add x86_64-apple-darwin

# Configure Cargo for cross-compilation (x86_64)
RUN mkdir -p /root/.cargo && \
    echo '\
[target.x86_64-apple-darwin]\n\
linker = "x86_64-apple-darwin20.4-clang"\n\
ar = "x86_64-apple-darwin20.4-ar"\n\
' > /root/.cargo/config.toml

# Copy the source code
COPY . .

# Build for x86_64 macOS
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/build/target \
    cargo build --bin rivet-engine --release --target x86_64-apple-darwin && \
    mkdir -p /artifacts && \
    cp target/x86_64-apple-darwin/release/rivet-engine /artifacts/rivet-engine-x86_64-apple-darwin

# Default command to show help
CMD ["ls", "-la", "/artifacts"]
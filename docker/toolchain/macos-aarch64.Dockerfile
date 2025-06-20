# syntax=docker/dockerfile:1.4
FROM rust:1.82.0 AS base

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

# Common environment variables for cross-compilation
ENV MACOSX_DEPLOYMENT_TARGET=10.7 \
    # Skip aws-lc-rs with rustls certs config when building for macOS
    RUSTFLAGS="--cfg tokio_unstable --cfg rustls_native_certs --cfg aws_lc_rs" \
    CARGO_FEATURE_RUSTLS_NATIVE_CERTS=0 \
    CARGO_RUSTLS_NATIVE_CERTS=0 \
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

# Set environment variables for ARM64 cross-compilation
ENV CARGO_TARGET_AARCH64_APPLE_DARWIN_LINKER=aarch64-apple-darwin20.4-clang \
    CC_aarch64_apple_darwin=aarch64-apple-darwin20.4-clang \
    CXX_aarch64_apple_darwin=aarch64-apple-darwin20.4-clang++

# Copy the source code
COPY . .

# Build for ARM64 macOS
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/build/target \
    cargo build --bin rivet --release --target aarch64-apple-darwin && \
    mkdir -p /artifacts && \
    cp target/aarch64-apple-darwin/release/rivet /artifacts/rivet-aarch64-apple-darwin

# Default command to show help
CMD ["ls", "-la", "/artifacts"]
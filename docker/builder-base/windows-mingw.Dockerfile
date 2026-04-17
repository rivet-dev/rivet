# syntax=docker/dockerfile:1.10.0
# Base image for Windows (MinGW) cross-compilation.
# Used for both rivet-engine and rivetkit-napi addon builds.
# Pre-bakes MinGW-w64, Rust target, Node.js 22, napi-rs CLI.
#
# Build & push: scripts/docker-builder-base/build-push.sh windows-mingw
FROM rust:1.89.0-bookworm

RUN apt-get update && apt-get install -y --no-install-recommends \
    llvm-14-dev \
    libclang-14-dev \
    clang-14 \
    lld \
    git-lfs \
    gcc-mingw-w64-x86-64 \
    g++-mingw-w64-x86-64 \
    binutils-mingw-w64-x86-64 \
    ca-certificates \
    wget \
    curl && \
    curl -fsSL https://deb.nodesource.com/setup_22.x | bash - && \
    apt-get install -y --no-install-recommends nodejs && \
    corepack enable && \
    npm install -g @napi-rs/cli && \
    rm -rf /var/lib/apt/lists/*

# Use the POSIX MinGW-w64 threading model (required for Rust's std::thread).
RUN update-alternatives --set x86_64-w64-mingw32-gcc /usr/bin/x86_64-w64-mingw32-gcc-posix && \
    update-alternatives --set x86_64-w64-mingw32-g++ /usr/bin/x86_64-w64-mingw32-g++-posix

RUN rustup target add x86_64-pc-windows-gnu

# Install sccache (prebuilt musl binary).
RUN SCCACHE_VERSION=v0.8.2 && \
    wget -q https://github.com/mozilla/sccache/releases/download/${SCCACHE_VERSION}/sccache-${SCCACHE_VERSION}-x86_64-unknown-linux-musl.tar.gz && \
    tar -xzf sccache-${SCCACHE_VERSION}-x86_64-unknown-linux-musl.tar.gz && \
    mv sccache-${SCCACHE_VERSION}-x86_64-unknown-linux-musl/sccache /usr/local/bin/sccache && \
    chmod +x /usr/local/bin/sccache && \
    rm -rf sccache-${SCCACHE_VERSION}-x86_64-unknown-linux-musl*

RUN mkdir -p /root/.cargo && \
    echo '[target.x86_64-pc-windows-gnu]\nlinker = "x86_64-w64-mingw32-gcc"\n' > /root/.cargo/config.toml

ENV CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc \
    CC_x86_64_pc_windows_gnu=x86_64-w64-mingw32-gcc \
    CXX_x86_64_pc_windows_gnu=x86_64-w64-mingw32-g++ \
    CC_x86_64-pc-windows-gnu=x86_64-w64-mingw32-gcc \
    CXX_x86_64-pc-windows-gnu=x86_64-w64-mingw32-g++ \
    LIBCLANG_PATH=/usr/lib/llvm-14/lib \
    CLANG_PATH=/usr/bin/clang-14 \
    CARGO_INCREMENTAL=0 \
    CARGO_NET_GIT_FETCH_WITH_CLI=true \
    COREPACK_ENABLE_DOWNLOAD_PROMPT=0

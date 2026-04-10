# Base image for Windows (MinGW) cross-compilation.
# Pre-bakes MinGW-w64 toolchain, Rust target, and Node.js.
#
# Build & push: scripts/docker-builder-base/build-push.sh windows-mingw
# syntax=docker/dockerfile:1.10.0
FROM rust:1.89.0-bookworm

RUN apt-get update && apt-get install -y \
    llvm-14-dev \
    libclang-14-dev \
    clang-14 \
    git-lfs \
    gcc-mingw-w64-x86-64 \
    g++-mingw-w64-x86-64 \
    binutils-mingw-w64-x86-64 \
    ca-certificates \
    curl && \
    curl -fsSL https://deb.nodesource.com/setup_22.x | bash - && \
    apt-get install -y nodejs && \
    corepack enable && \
    rm -rf /var/lib/apt/lists/*

# Switch MinGW-w64 to POSIX threading model
RUN update-alternatives --set x86_64-w64-mingw32-gcc /usr/bin/x86_64-w64-mingw32-gcc-posix && \
    update-alternatives --set x86_64-w64-mingw32-g++ /usr/bin/x86_64-w64-mingw32-g++-posix

RUN rustup target add x86_64-pc-windows-gnu

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

# syntax=docker/dockerfile:1.10.0
FROM rust:1.89.0-bookworm

# Install dependencies for cargo-xwin (MSVC cross-compilation from Linux)
RUN apt-get update && apt-get install -y \
    git-lfs \
    clang \
    llvm \
    lld \
    curl && \
    curl -fsSL https://deb.nodesource.com/setup_22.x | bash - && \
    apt-get install -y nodejs && \
    npm install -g @napi-rs/cli && \
    rm -rf /var/lib/apt/lists/*

# Install cargo-xwin for MSVC cross-compilation
RUN cargo install cargo-xwin

RUN rustup target add x86_64-pc-windows-msvc

ENV CARGO_INCREMENTAL=0 \
    CARGO_NET_GIT_FETCH_WITH_CLI=true

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

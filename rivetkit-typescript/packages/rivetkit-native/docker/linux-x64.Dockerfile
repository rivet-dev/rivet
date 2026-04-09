# syntax=docker/dockerfile:1.10.0
FROM rust:1.89.0-bookworm

# Install Node.js and napi-rs dependencies
RUN apt-get update && apt-get install -y \
    git-lfs \
    clang \
    llvm-dev \
    libclang-dev \
    curl && \
    curl -fsSL https://deb.nodesource.com/setup_22.x | bash - && \
    apt-get install -y nodejs && \
    npm install -g @napi-rs/cli && \
    rm -rf /var/lib/apt/lists/*

ENV CARGO_INCREMENTAL=0 \
    CARGO_NET_GIT_FETCH_WITH_CLI=true

WORKDIR /build

COPY . .

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/build/target \
    cd rivetkit-typescript/packages/rivetkit-native && \
    napi build --platform --release --target x86_64-unknown-linux-gnu && \
    mkdir -p /artifacts && \
    cp rivetkit-native.linux-x64-gnu.node /artifacts/

CMD ["ls", "-la", "/artifacts"]

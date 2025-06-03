# syntax=docker/dockerfile:1.4
FROM rust:1.82.0

# Install dependencies
RUN apt-get update && apt-get install -y \
    git-lfs \
    protobuf-compiler \
    gcc-mingw-w64-x86-64 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Install target
RUN rustup target add x86_64-pc-windows-gnu

# Configure Cargo for Windows cross-compilation
RUN mkdir -p /root/.cargo && \
    echo '\
[target.x86_64-pc-windows-gnu]\n\
linker = "x86_64-w64-mingw32-gcc"\n\
' > /root/.cargo/config.toml

# Set environment variables for cross-compilation
ENV CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc \
    RUSTFLAGS="--cfg tokio_unstable" \
    CARGO_INCREMENTAL=0 \
    CARGO_NET_GIT_FETCH_WITH_CLI=true

# Set working directory
WORKDIR /build

# Copy the source code
COPY . .

# Build for Windows
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/build/target \
    cargo build --bin rivet --release --target x86_64-pc-windows-gnu && \
    mkdir -p /artifacts && \
    cp target/x86_64-pc-windows-gnu/release/rivet.exe /artifacts/rivet-x86_64-pc-windows-gnu.exe

# Default command to show help
CMD ["ls", "-la", "/artifacts"]

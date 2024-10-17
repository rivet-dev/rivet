FROM clux/muslrust:1.81.0-stable AS rust

RUN ln -s /bin/g++ /bin/musl-g++ && \
	ln -s /bin/gcc-ar /bin/musl-ar

WORKDIR /app
COPY . .
RUN \
	--mount=type=cache,target=/root/.cargo/git \
	--mount=type=cache,target=/root/.cargo/registry \
	--mount=type=cache,target=/app/target \
	RUSTFLAGS="--cfg tokio_unstable" cargo build --release --package pegboard-manager --bin pegboard-manager && \
	mkdir -p /app/dist && \
	cp /app/target/x86_64-unknown-linux-musl/release/pegboard-manager /app/dist/pegboard-manager

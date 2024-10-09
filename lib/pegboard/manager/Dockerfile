FROM clux/muslrust:1.80.0-stable AS rust

RUN ln -s /bin/g++ /bin/musl-g++

WORKDIR /app
COPY lib/ lib/
COPY svc/ svc/
COPY errors/ errors/
COPY Bolt.toml Bolt.toml
COPY proto/ proto/
RUN \
	--mount=type=cache,target=/root/.cargo/git \
	--mount=type=cache,target=/root/.cargo/registry \
	--mount=type=cache,target=/app/lib/pegboard/target \
	cd lib/pegboard/manager && RUSTFLAGS="--cfg tokio_unstable" cargo build --release && mkdir /usr/bin/rivet && mv /app/lib/pegboard/target/x86_64-unknown-linux-musl/release/pegboard-manager /usr/bin/rivet/

# Create an empty image and copy binaries into it to minimize the size of the image
FROM scratch
COPY --from=rust /usr/bin/rivet/ /

# Allows `docker create` to work even though this fails
CMD [""]

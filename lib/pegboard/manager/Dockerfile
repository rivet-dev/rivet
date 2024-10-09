FROM clux/muslrust:1.80.0-stable

RUN ln -s /bin/g++ /bin/musl-g++

WORKDIR /app
COPY lib/ lib/
COPY svc/ svc/
COPY errors/ errors/
COPY Bolt.toml Bolt.toml
COPY proto/ proto/
RUN cd lib/pegboard/manager && RUSTFLAGS="--cfg tokio_unstable" cargo build --release

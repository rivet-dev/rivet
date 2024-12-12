# Troubleshooting

## `failed to solve`

The following indicates that you've ran out of disk space.

```
84.84 rustc-LLVM ERROR: IO failure on output stream: No space left on device
...etc...
90.32 error: could not compile `xxxx` (lib); 1 warning emitted
------
failed to solve: process "/bin/sh -c RUSTFLAGS=\"--cfg tokio_unstable\" cargo build --bin rivet-server && \tmv target/debug/rivet-server /usr/bin/rivet-server && \tmkdir /etc/rivet-server" did not complete successfully: exit code: 101
```

## `error: linking with `cc` failed: exit status: 1`

The following indicates that the process was force killed (`signal 9`), which usually indicates you've ran out of memory.

```
2.246    Compiling rivet-server v0.0.1 (/app/packages/infra/server)
55.18 error: linking with `cc` failed: exit status: 1
...etc...
55.18   = note: collect2: fatal error: ld terminated with signal 9 [Killed]
55.18           compilation terminated.
55.18           
55.18 
55.35 error: could not compile `rivet-server` (bin "rivet-server") due to 1 previous error
```

If you're using `docker-compose up --build`, try building one service at a time. Building multiple Rust binaries at the same time will eat a lot of memory.


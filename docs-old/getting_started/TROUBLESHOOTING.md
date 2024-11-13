# Troubleshooting

## `failed to solve`

### `rustc-LLVM ERROR: IO failure on output stream: No space left on device`

The following 

```
84.84 rustc-LLVM ERROR: IO failure on output stream: No space left on device
...etc...
90.32 error: could not compile `xxxx` (lib); 1 warning emitted
------
failed to solve: process "/bin/sh -c RUSTFLAGS=\"--cfg tokio_unstable\" cargo build --bin rivet-server && \tmv target/debug/rivet-server /usr/bin/rivet-server && \tmkdir /etc/rivet-server" did not complete successfully: exit code: 101
```

## `default user not found`

TODO


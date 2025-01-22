# CLI

## Development

For quick iteration, use `cargo run --bin rivet -- ...etc...`. For example, to test `rivet deploy` on an example, run:

```
cd examples/javascript/counter
cargo run --bin rivet -- deploy
```

For installing system-wide, run:

```
cargo install --path packages/toolchain/cli
```


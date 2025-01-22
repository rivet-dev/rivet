# Examples & Templates

These are examples & templates for Rivet.

## Development

Examples can be deployed to either a local Rivet cluster or a production Rivet cluster.

If using bleeding edge examples, it's recommended to use `cargo run` to auto-build the Rivet CLI. For example, to deploy the counter example:

```
cd examples/javascript/counter
cargo run --bin rivet -- deploy
```


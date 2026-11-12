# Hello World (Rust)

A minimal example demonstrating RivetKit in Rust with a real-time counter shared across multiple clients. This is the Rust translation of the [`hello-world`](../hello-world) example.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/hello-world-rust
cargo build -p rivet-engine
RIVET_ENGINE_BINARY_PATH=../../target/debug/rivet-engine cargo run
```

See the [Rust Quickstart](/docs/actors/quickstart/rust) for creating the actor and connecting clients.

## Features

- **Persistent data**: The counter is stored in the actor's SQLite database
- **Real-time updates**: Counter values synchronized across all connected clients via events
- **Multiple actor instances**: Each counter key creates a separate actor instance
- **Typed Rust runtime**: Built on the `rivetkit` crate's `Actor` and `Handler` traits

## Implementation

This example demonstrates the core RivetKit concepts with a simple counter:

- **Actor Definition** ([`src/lib.rs`](https://github.com/rivet-dev/rivet/tree/main/examples/hello-world-rust/src/lib.rs)): A `counter` actor with a SQLite-backed count, an `increment` action, and a `newCount` broadcast event

## Resources

Read more about [actions](/docs/actors/actions), [SQLite](/docs/actors/sqlite), and [events](/docs/actors/events).

## License

MIT

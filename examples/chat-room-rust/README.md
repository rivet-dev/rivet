# Chat Room (Rust)

Example project demonstrating real-time messaging and actor state management in Rust. This is the Rust translation of the [`chat-room`](../chat-room) example.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/chat-room-rust
cargo build -p rivet-engine
RIVET_ENGINE_BINARY_PATH=../../target/debug/rivet-engine cargo run
```

See the [Rust Quickstart](/docs/actors/quickstart/rust) for creating the actor and connecting clients.

## Features

- **Real-time messaging**: Broadcast messages to all connected clients instantly
- **Persistent chat history**: Messages automatically saved in actor state across restarts
- **Multiple chat rooms**: Each room key is a separate actor instance with isolated state
- **Event-driven architecture**: Use actor events to push updates to clients in real-time

## Implementation

The chat room demonstrates core Rivet Actor patterns for real-time communication:

- **Actor Definition** ([`src/lib.rs`](https://github.com/rivet-dev/rivet/tree/main/examples/chat-room-rust/src/lib.rs)): Defines the `chatRoom` actor with a SQLite-backed message history, `sendMessage` and `getHistory` actions, and a `newMessage` event

## Resources

Read more about [actions](/docs/actors/actions), [state](/docs/actors/state), and [events](/docs/actors/events).

## License

MIT

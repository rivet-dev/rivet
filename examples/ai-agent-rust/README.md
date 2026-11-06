# AI Agent (Rust)

A minimal AI agent built with RivetKit in Rust. Each conversation is its own Rivet Actor that stores its history in SQLite and sends the full history to the model on every message, so the agent remembers every turn across sleep and restarts.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/ai-agent-rust
cargo build -p rivet-engine
OPENAI_API_KEY=sk-... RIVET_ENGINE_BINARY_PATH=../../target/debug/rivet-engine cargo run
```

See the [Rust Quickstart](/docs/actors/quickstart/rust) for connecting clients.

## Prerequisites

- An OpenAI API key in `OPENAI_API_KEY`. Set `OPENAI_BASE_URL` to use another OpenAI-compatible API.

## Features

- **One actor per conversation**: Each agent key gets its own actor and its own history
- **Persistent memory**: Every user and assistant turn is stored in the actor's SQLite database
- **Streaming replies**: Each token is broadcast as a `response` event with the reply so far, and each new message as a `messageAdded` event
- **Typed Rust runtime**: Built on the `rivetkit` crate's `Actor` and `Handler` traits

## Implementation

- **Actor Definition** ([`src/agent.rs`](https://github.com/rivet-dev/rivet/tree/main/examples/ai-agent-rust/src/agent.rs)): The `agent` actor and its `sendMessage` and `getHistory` handlers
- **Helpers** ([`src/helpers.rs`](https://github.com/rivet-dev/rivet/tree/main/examples/ai-agent-rust/src/helpers.rs)): The actions and events clients use, and the streaming OpenAI call made with [`async-openai`](https://docs.rs/async-openai)

## Resources

Read more about [actions](/docs/actors/actions), [SQLite](/docs/actors/sqlite), and [events](/docs/actors/events).

## License

MIT

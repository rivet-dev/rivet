# Sandbox Coding Agent

Example project demonstrating a queue-free Rivet Actor chat UI backed by the new `rivetkit/sandbox` actor.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/sandbox-coding-agent
pnpm install
pnpm dev
```

## Features

- Uses `rivetkit/sandbox` instead of constructing `SandboxAgent` clients directly
- One sandbox actor per agent key
- Built-in transcript persistence through the sandbox actor
- Provider selection with `SANDBOX_PROVIDER=docker|daytona|e2b`
- Live UI updates via Rivet Actor events

## Prerequisites

- For local Docker usage, install Docker and leave `SANDBOX_PROVIDER` unset
- For Daytona, set `SANDBOX_PROVIDER=daytona` and `DAYTONA_API_KEY`
- For E2B, set `SANDBOX_PROVIDER=e2b` and `E2B_API_KEY`
- Set the model provider credentials needed inside the sandbox, such as `OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, or `CODEX_API_KEY`

## Implementation

The example keeps a thin UI-facing agent actor, but the sandbox lifecycle and sandbox-agent API now live behind `sandboxActor(...)`.

- **Sandbox actor integration** ([`src/actors.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/sandbox-coding-agent/src/actors.ts))
- **Frontend orchestration** ([`frontend/App.tsx`](https://github.com/rivet-dev/rivet/tree/main/examples/sandbox-coding-agent/frontend/App.tsx))
- **Server entry point** ([`src/server.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/sandbox-coding-agent/src/server.ts))

## Resources

Read more about [Rivet Actors](https://rivet.dev/docs/actors), [actions](https://rivet.dev/docs/actors/actions), [events](https://rivet.dev/docs/actors/events), and [queues](https://rivet.dev/docs/actors/queues).

## License

MIT

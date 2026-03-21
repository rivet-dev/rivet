> **Note:** This is the Vercel-optimized version of the [sandbox-coding-agent](../sandbox-coding-agent) example.
> It uses the `hono/vercel` adapter and is configured for Vercel deployment.

[![Deploy with Vercel](https://vercel.com/button)](https://vercel.com/new/clone?repository-url=https%3A%2F%2Fgithub.com%2Frivet-dev%2Frivet%2Ftree%2Fmain%2Fexamples%2Fsandbox-coding-agent-vercel&project-name=sandbox-coding-agent-vercel)

# Sandbox Coding Agent

Example project demonstrating a Rivet Actor chat UI backed by the new `rivetkit/sandbox` actor.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/sandbox-coding-agent-vercel
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

- On Vercel, use `SANDBOX_PROVIDER=daytona` or `SANDBOX_PROVIDER=e2b`
- For Daytona, set `DAYTONA_API_KEY`
- For E2B, set `E2B_API_KEY`
- Set the model provider credentials needed inside the sandbox, such as `OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, or `CODEX_API_KEY`

## Implementation

The example keeps a thin UI-facing agent actor, but the sandbox lifecycle and sandbox-agent API now live behind `sandboxActor(...)`.

- **Sandbox actor integration** ([`src/actors.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/sandbox-coding-agent-vercel/src/actors.ts))
- **Frontend orchestration** ([`frontend/App.tsx`](https://github.com/rivet-dev/rivet/tree/main/examples/sandbox-coding-agent-vercel/frontend/App.tsx))
- **Server entry point** ([`src/server.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/sandbox-coding-agent-vercel/src/server.ts))

## Resources

Read more about [Rivet Actors](https://rivet.dev/docs/actors), [actions](https://rivet.dev/docs/actors/actions), and [events](https://rivet.dev/docs/actors/events).

## License

MIT

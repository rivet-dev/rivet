> **Note:** This is the Vercel-optimized version of the [sandbox-coding-agent](../sandbox-coding-agent) example.
> It uses the `hono/vercel` adapter and is configured for Vercel deployment.

[![Deploy with Vercel](https://vercel.com/button)](https://vercel.com/new/clone?repository-url=https%3A%2F%2Fgithub.com%2Frivet-gg%2Frivet%2Ftree%2Fmain%2Fexamples%2Fsandbox-coding-agent-vercel&project-name=sandbox-coding-agent-vercel)

# Sandbox Coding Agent

Example project demonstrating queue-driven Rivet Actor sessions that control a Sandbox Agent coding runtime.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/sandbox-coding-agent
npm install
npm run dev
```


## Features

- Actor-per-agent pattern with a coordinating manager Rivet Actor
- Queue-based intake using `for await` over `c.queue.iter(...)` inside the run loop
- Sandbox Agent SDK sessions per agent with streamed output
- Persistent history stored in Rivet Actor state
- Live status updates via events and polling

## Prerequisites

- Sandbox Agent endpoint configured with `SANDBOX_AGENT_URL` and `SANDBOX_AGENT_TOKEN` if you are not running locally
- API key for the coding agent you choose inside the Sandbox Agent environment, such as `OPENAI_API_KEY` for `codex`

## Implementation

Each AI agent Rivet Actor creates or reuses a Sandbox Agent session, streams turn events, and broadcasts deltas back to the UI.

- **Sandbox agent integration** ([`src/actors.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/sandbox-coding-agent/src/actors.ts))
- **Frontend orchestration** ([`frontend/App.tsx`](https://github.com/rivet-dev/rivet/tree/main/examples/sandbox-coding-agent/frontend/App.tsx))
- **Server entry point** ([`src/server.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/sandbox-coding-agent/src/server.ts))

## Resources

Read more about [queues](https://rivet.dev/docs/actors/queues), [run handlers](https://rivet.dev/docs/actors/run), [state](https://rivet.dev/docs/actors/state), and [events](https://rivet.dev/docs/actors/events).

## License

MIT

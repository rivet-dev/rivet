> **Note:** This is the Vercel-optimized version of the [ai-agent](../ai-agent) example.
> It uses the `hono/vercel` adapter and is configured for Vercel deployment.

[![Deploy with Vercel](https://vercel.com/button)](https://vercel.com/new/clone?repository-url=https%3A%2F%2Fgithub.com%2Frivet-gg%2Frivet%2Ftree%2Fmain%2Fexamples%2Fai-agent-vercel&project-name=ai-agent-vercel)

# AI Agent

Example project demonstrating queue-driven Rivet Actor AI agents with streaming Vercel AI SDK responses.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/ai-agent
pnpm install
pnpm dev
```

## Features

- Actor-per-agent pattern with a coordinating manager Rivet Actor
- Queue-based intake using `c.queue.next` inside the run loop
- Streaming AI responses sent to the UI as they arrive
- Persistent history stored in Rivet Actor state
- Live status updates via events and polling

## Prerequisites

- OpenAI API key set as `OPENAI_API_KEY`

## Implementation

The AgentManager creates and tracks agent actors, while each AI agent Rivet Actor consumes queue messages in `run` and streams responses with the Vercel AI SDK.

- **Actor definitions and queues** ([`src/actors.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/ai-agent/src/actors.ts))
- **Frontend orchestration** ([`frontend/App.tsx`](https://github.com/rivet-dev/rivet/tree/main/examples/ai-agent/frontend/App.tsx))
- **Server entry point** ([`src/server.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/ai-agent/src/server.ts))

## Resources

Read more about [queues](https://rivet.dev/docs/actors/queues), [run handlers](https://rivet.dev/docs/actors/run), [state](https://rivet.dev/docs/actors/state), and [events](https://rivet.dev/docs/actors/events).

## License

MIT

> **Note:** This is the Vercel-optimized version of the [agent-orchestration](../agent-orchestration) example.
> It uses the `hono/vercel` adapter and is configured for Vercel deployment.

[![Deploy with Vercel](https://vercel.com/button)](https://vercel.com/new/clone?repository-url=https%3A%2F%2Fgithub.com%2Frivet-gg%2Frivet%2Ftree%2Fmain%2Fexamples%2Fagent-orchestration-vercel&project-name=agent-orchestration-vercel)

# Agent Orchestration

Example project demonstrating queue-driven Rivet Actor orchestration with streaming AI responses.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/agent-orchestration
pnpm install
pnpm dev
```

## Features

- **Actor-per-agent pattern**: Each agent runs as its own Rivet Actor instance managed by an AgentManager coordinator
- **Queue-based intake**: The `run` hook pulls messages with `c.queue.next` to drive background work
- **Streaming responses**: Agents broadcast response events as text is generated
- **Persistent history**: Conversation history lives in actor state and is exposed via `getHistory`
- **Live status updates**: Agent status can be polled with `getStatus` and streamed to clients

## Prerequisites

- OpenAI API key set as `OPENAI_API_KEY`

## Implementation

The AgentManager creates and tracks agent actors, while each agent Rivet Actor uses a queue loop in `run` to process messages and stream responses.

- **Actor definitions and queues** ([`src/actors.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/agent-orchestration/src/actors.ts)): Defines the AgentManager, per-agent actors, and response events
- **Frontend orchestration** ([`frontend/App.tsx`](https://github.com/rivet-dev/rivet/tree/main/examples/agent-orchestration/frontend/App.tsx)): Creates agents, sends queue messages, and renders streaming responses
- **Server entry point** ([`src/server.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/agent-orchestration/src/server.ts)): Connects the HTTP handler for RivetKit

## Resources

Read more about [lifecycle hooks](https://rivet.dev/docs/actors/lifecycle), [actions](https://rivet.dev/docs/actors/actions), [state](https://rivet.dev/docs/actors/state), and [events](https://rivet.dev/docs/actors/events).

## License

MIT

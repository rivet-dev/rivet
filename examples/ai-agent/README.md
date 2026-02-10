# AI Agent

Example project demonstrating queue-driven Rivet Actor AI agents with streaming Vercel AI SDK responses.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/ai-agent
npm install
npm run dev
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

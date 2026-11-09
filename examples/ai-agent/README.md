# AI Agent

Example project demonstrating Rivet Actor AI agents that answer each message in order and stream Vercel AI SDK responses.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/ai-agent
npm install
npm run dev
```


## Features

- Actor-per-agent pattern with a coordinating manager Rivet Actor
- Durable message intake: a `sendMessage` action saves each message to SQLite before it returns
- Ordered replies from a background drain started by the action and by `onWake`, one model call at a time
- Streaming AI responses sent to the UI as they arrive
- Persistent history stored in the Rivet Actor's SQLite database
- Live status updates via events and polling

## Prerequisites

- OpenAI API key set as `OPENAI_API_KEY`

## Implementation

The AgentManager creates and tracks agent actors. Each AI agent Rivet Actor saves incoming messages as pending, answers them oldest first with the Vercel AI SDK, and saves each reply together with the message it answers. Pending messages are answered after a restart, so a message sent before a crash is not lost.

- **Actor definitions and server entry point** ([`src/index.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/ai-agent/src/index.ts))
- **Frontend orchestration** ([`frontend/App.tsx`](https://github.com/rivet-dev/rivet/tree/main/examples/ai-agent/frontend/App.tsx))

## Resources

Read more about [actions](https://rivet.dev/docs/actors/actions), [lifecycle hooks](https://rivet.dev/docs/actors/lifecycle), [SQLite](https://rivet.dev/docs/actors/sqlite), and [events](https://rivet.dev/docs/actors/events).

## License

MIT

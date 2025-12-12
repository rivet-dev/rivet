# AI Agent with Durable Streams (Experimental)

Example project demonstrating how to build an AI agent that communicates through durable streams for reliable message delivery and persistence.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/experimental-durable-streams-ai-agent
npm install
npm run dev
```


## Features

- **Durable message delivery**: Prompts and responses flow through durable streams, ensuring reliable delivery even if components restart
- **Streaming AI responses**: AI responses stream in real-time through durable streams to the frontend
- **Actor-based processing**: AI agent runs as a Rivet Actor with persistent state tracking processed prompts

## Prerequisites

- Anthropic API Key (set as `ANTHROPIC_API_KEY` environment variable)
- (Optional) Durable Streams Test UI for inspecting streams and debugging message flow:
  ```sh
  git clone https://github.com/durable-streams/durable-streams.git
  cd durable-streams/packages/test-ui
  pnpm dev
  ```

## Implementation

The architecture uses two durable streams per conversation:

1. **Prompt Stream** (`/conversations/{id}/prompts`): Frontend writes user messages, actor consumes them
2. **Response Stream** (`/conversations/{id}/responses`): Actor writes AI response chunks, frontend consumes them

Key implementation details:

- **AI Agent Actor** ([`src/backend/registry.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/experimental-durable-streams-ai-agent/src/backend/registry.ts)): Defines the `aiAgent` actor that consumes prompts from durable streams and writes AI responses back
- **Frontend** ([`src/frontend/App.tsx`](https://github.com/rivet-dev/rivet/tree/main/examples/experimental-durable-streams-ai-agent/src/frontend/App.tsx)): React chat UI that reads/writes to durable streams
- **Streams Server** ([`src/streams-server/server.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/experimental-durable-streams-ai-agent/src/streams-server/server.ts)): In-memory durable streams server for development
- **Message Types** ([`src/backend/types.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/experimental-durable-streams-ai-agent/src/backend/types.ts)): TypeScript types for prompt and response messages

## Resources

- [Durable Streams](https://github.com/durable-streams/durable-streams) - Reliable message delivery and persistence
- [Architecture Slides](https://link.excalidraw.com/p/readonly/K9IN9fZ41gkLfPedMbZg) - Visual overview of the durable streams architecture

## License

MIT

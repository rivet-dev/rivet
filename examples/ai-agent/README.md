# AI Agent Chat

Example project demonstrating AI agent integration.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/ai-agent
npm install
npm run dev
```


## Features

- **AI SDK integration**: Use Vercel AI SDK with OpenAI within Rivet Actors
- **Persistent conversation state**: Message history automatically persisted across actor restarts
- **Real-time updates**: Broadcast AI responses to connected clients using actor events
- **Tool calling**: Integrate custom tools (weather lookup) that AI can invoke

## Implementation

The AI agent is implemented as a Rivet Actor that maintains conversation state and integrates with OpenAI. Key implementation details:

- **Actor Definition** ([`src/backend/registry.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/ai-agent/src/backend/registry.ts)): Defines the `aiAgent` actor with persistent message history
- **Custom Tools** ([`src/backend/my-tools.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/ai-agent/src/backend/my-tools.ts)): Implements the weather lookup tool that the AI can invoke
- **Message Types** ([`src/backend/types.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/ai-agent/src/backend/types.ts)): TypeScript types for message structure

## Prerequisites

- OpenAI API Key (set as `OPENAI_API_KEY` environment variable)

## Resources

Read more about [actions](/docs/actors/actions), [state](/docs/actors/state), and [events](/docs/actors/events).

## License

MIT

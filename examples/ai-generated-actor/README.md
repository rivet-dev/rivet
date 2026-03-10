# AI-Generated Actor

Use an AI chat to generate and iterate on Rivet Actor code, then deploy and test it live.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/ai-generated-actor
npm install
npm run dev
```

## Prerequisites

- OpenAI API key set as `OPENAI_API_KEY`
- Build `sandboxed-node` and make it resolvable (or set `RIVETKIT_DYNAMIC_SECURE_EXEC_SPECIFIER`)

## Features

- AI-driven code generation using GPT-5 via the Vercel AI SDK with streaming responses
- Dynamic actor loading via `dynamicActor` from `rivetkit/dynamic`
- Per-key isolation where each actor key has its own AI agent, generated code, and dynamic actor instance
- Generic actor interface to call arbitrary actions on the generated actor
- Three-column layout: chat, generated code, and actor testing interface

## Implementation

The project uses two actors defined in [`src/actors.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/ai-generated-actor/src/actors.ts):

- `codeAgent` maintains chat history and generated code in actor state. It processes messages via a queue and streams AI responses using the Vercel AI SDK, extracting code blocks from the response to update the current actor source.
- `dynamicRunner` is a dynamic actor that loads its source code from the `codeAgent` with the matching key, executing the AI-generated code in a sandboxed isolate.

The server in [`src/server.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/ai-generated-actor/src/server.ts) exposes proxy endpoints for calling actions on the dynamic actor by name.

The frontend in [`frontend/App.tsx`](https://github.com/rivet-dev/rivet/tree/main/examples/ai-generated-actor/frontend/App.tsx) provides a three-column interface for chatting with the AI, viewing generated code, and testing the deployed actor.

## Resources

Read more about [dynamic actors](https://rivet.dev/docs/actors/ai-and-user-generated-actors), [queues](https://rivet.dev/docs/actors/queues), [events](https://rivet.dev/docs/actors/events), and [state](https://rivet.dev/docs/actors/state).

## License

MIT

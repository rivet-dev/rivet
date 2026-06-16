# AI Agent (Effect)

A stateful AI agent actor built with the [Effect](https://effect.website) SDK for Rivet Actors and [Effect AI](https://effect.website/docs/ai/introduction/). The agent persists its conversation history in actor state, so it remembers every turn across calls and restarts.

## Prerequisites

Running against a real model needs an OpenAI API key:

```sh
export OPENAI_API_KEY=sk-...
```

The test suite needs neither a key nor network access: it runs a real in-process mock LLM server instead.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/ai-agent-effect
npm install
npm run dev
```

In a separate terminal, run the client against the server:

```sh
npm run client
```

## Features

- **Persistent conversation memory**: Every user and assistant turn is stored in persisted actor state (a `Schema.Array` of role/content messages), so the model sees the full history on each call, even after the actor sleeps or restarts
- **Effect AI integration**: The agent calls `LanguageModel.generateText` from `effect/unstable/ai` with the running history as the prompt
- **Swappable model via Layer**: The actor requires the `LanguageModel` service but never constructs it. The concrete model is provided as an Effect `Layer` where the actor is composed: a real OpenAI model in `dev`, a mock model in tests
- **Effect-native actor**: Defined with `Actor.make` and implemented with `toLayer`, returning action handlers from an Effect wake scope
- **Typed errors across the wire**: `SendMessage` declares an `EmptyMessageError` that arrives on the caller as a real tagged instance, caught with `Effect.catchTag`

## Implementation

The actor is split into a public contract and a server-only implementation:

- **Contract** ([`src/actors/agent/api.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/ai-agent-effect/src/actors/agent/api.ts)): Declares the `Agent` actor, the `Message` type, the `SendMessage` / `GetHistory` actions, and the `EmptyMessageError`
- **Implementation** ([`src/actors/agent/live.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/ai-agent-effect/src/actors/agent/live.ts)): Implements the wake scope, the conversation-history state schema, and the action handlers that call the LLM
- **Model wiring** ([`src/model.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/ai-agent-effect/src/model.ts)): Composes a `Layer<LanguageModel>` from `OpenAiLanguageModel`, `OpenAiClient`, and `FetchHttpClient`
- **Server** ([`src/main.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/ai-agent-effect/src/main.ts)): Provides the model Layer to the actor layer and serves it with `Registry.serve`
- **Client** ([`src/client.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/ai-agent-effect/src/client.ts)): An Effect client using the typed `Agent.client` accessor
- **Test** ([`tests/agent.test.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/ai-agent-effect/tests/agent.test.ts)): Runs the actor against a real engine and a real in-process mock LLM server, swapping in a mock model Layer

## Resources

Read more about [actions](/docs/actors/actions), [state](/docs/actors/state), and [the Effect quickstart](/docs/actors/quickstart/effect).

## License

MIT

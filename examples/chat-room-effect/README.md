# Chat Room (Effect)

Example project demonstrating a real-time chat room built with the [Effect](https://effect.website) SDK for Rivet Actors.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/chat-room-effect
npm install
npm run dev
```

In a separate terminal, run a client against the server:

```sh
npm run client      # Effect client
npm run client:raw  # plain RivetKit client
```

## Features

- **Effect-native actors**: Define actors with `Actor.make` and implement them with `toLayer`, composing actor logic from Effect `Layer`s and services
- **Typed action protocols**: Actions are standalone `Action.make` values with `effect/Schema` payloads, successes, and errors validated end to end
- **Typed domain errors**: `MemberNotInRoomError` and `BannedWordsError` flow through the action error channel and are caught by tag on the client
- **Actor-to-actor RPC**: The `ChatRoom` actor calls a separate `Moderator` actor to screen messages, using the same client API as client-to-actor calls
- **Persistent state and SQLite**: Room membership lives in persisted actor state while message history is stored in the actor's SQLite database
- **Scheduling**: A welcome message is scheduled after a member joins and dispatched back through the actor's own action

## Implementation

The example splits each actor into a public contract and a server-only implementation:

- **Chat room contract** ([`src/actors/chat-room/api.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/chat-room-effect/src/actors/chat-room/api.ts)): Declares the `ChatRoom` actor, its actions, and its typed errors
- **Chat room implementation** ([`src/actors/chat-room/live.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/chat-room-effect/src/actors/chat-room/live.ts)): Implements the wake scope, state schema, SQLite migration, and action handlers
- **Moderator** ([`src/actors/moderator/api.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/chat-room-effect/src/actors/moderator/api.ts), [`live.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/chat-room-effect/src/actors/moderator/live.ts)): A second actor that screens messages for banned words
- **Server** ([`src/main.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/chat-room-effect/src/main.ts)): Composes the actor layers and serves them with `Registry.serve`
- **Clients** ([`src/client.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/chat-room-effect/src/client.ts), [`src/client-raw.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/chat-room-effect/src/client-raw.ts)): An Effect client using the typed `ChatRoom.client` accessor, and a plain RivetKit client showing the same actors reached from non-Effect code

## Resources

Read more about [actions](/docs/actors/actions), [state](/docs/actors/state), [events](/docs/actors/events), and [the Effect quickstart](/docs/actors/quickstart/effect).

## License

MIT

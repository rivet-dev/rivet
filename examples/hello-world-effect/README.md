# Hello World (Effect)

Minimal counter actor built with the [Effect](https://effect.website) SDK for Rivet Actors.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/hello-world-effect
npm install
npm run dev
```

In a separate terminal, run the client against the server:

```sh
npm run client
```

## Features

- **Effect-native actors**: Define an actor with `Actor.make` and implement it with `toLayer`, returning action handlers from an Effect wake scope
- **Typed action protocols**: `Increment` and `GetCount` are standalone `Action.make` values with `Schema` payloads and successes validated end to end
- **Typed errors across the wire**: `Increment` declares a `NegativeAmountError` that arrives on the caller as a real tagged instance, caught with `Effect.catchTag`
- **Persistent state**: The counter value lives in persisted actor state, accessed through a `SubscriptionRef`-like `State` API
- **Events**: Each increment broadcasts the new count to every connected client

## Implementation

The actor is split into a public contract and a server-only implementation:

- **Contract** ([`src/actors/counter/api.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/hello-world-effect/src/actors/counter/api.ts)): Declares the `Counter` actor and its actions
- **Implementation** ([`src/actors/counter/live.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/hello-world-effect/src/actors/counter/live.ts)): Implements the wake scope, state schema, and action handlers
- **Server** ([`src/main.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/hello-world-effect/src/main.ts)): Composes the actor layer and serves it with `Registry.serve`
- **Client** ([`src/client.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/hello-world-effect/src/client.ts)): An Effect client using the typed `Counter.client` accessor

## Resources

Read more about [actions](/docs/actors/actions), [state](/docs/actors/state), [events](/docs/actors/events), and [the Effect quickstart](/docs/actors/quickstart/effect).

## License

MIT

# Actor Actions

Demonstrates how to define and call actions on Rivet Actors for RPC-style communication between actors and clients.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/actor-actions
npm install
npm run dev
```


## Features

- **Type-safe actor actions**: Define and call actions with full TypeScript type safety
- **Durable data in SQLite**: Save creation input in `onCreate`, then read and update it with SQL in each action
- **Cross-actor communication**: Create and interact with actors from within other actors
- **RPC-style patterns**: Call actor methods from client code with automatic type inference

## Implementation

This example demonstrates the fundamentals of defining and calling actor actions:

- **Actor Definition** ([`src/index.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/actor-actions/src/index.ts)): Shows how to define actions with parameters and return values, backed by each actor's SQLite database

## Resources

Read more about [actions](/docs/actors/actions), [state](/docs/actors/state), [SQLite](/docs/actors/sqlite), and [actor setup](/docs/setup).

## License

MIT

# Deno Example

Example project demonstrating basic actor state management and RPC calls using Deno runtime.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/deno
npm install
npm run dev
```


## Features

- **Deno runtime support**: Run Rivet Actors on Deno for modern JavaScript/TypeScript execution
- **Type-safe actions**: Define and call actor actions with full TypeScript type safety
- **Actor state management**: Persistent state automatically managed across actor lifecycle
- **RPC-style communication**: Call actor methods from client code with automatic serialization

## Implementation

This example demonstrates using Rivet Actors with the Deno runtime:

- **Actor Definition** ([`src/backend/registry.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/deno/src/backend/registry.ts)): Shows how to configure actors for Deno runtime

## Resources

Read more about [actions](/docs/actors/actions), [state](/docs/actors/state), and [setup](/docs/setup).

## License

MIT

> **Note:** This is the Vercel-optimized version of the [actor-actions](../actor-actions) example.
> It uses the `hono/vercel` adapter and is configured for Vercel deployment.

[![Deploy with Vercel](https://vercel.com/button)](https://vercel.com/new/clone?repository-url=https%3A%2F%2Fgithub.com%2Frivet-gg%2Frivet%2Ftree%2Fmain%2Fexamples%2Factor-actions-vercel&project-name=actor-actions-vercel)

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
- **Actor state management**: Initialize and persist actor state using `createState`
- **Cross-actor communication**: Create and interact with actors from within other actors
- **RPC-style patterns**: Call actor methods from client code with automatic type inference

## Implementation

This example demonstrates the fundamentals of defining and calling actor actions:

- **Actor Definition** ([`src/backend/registry.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/actor-actions/src/backend/registry.ts)): Shows how to define actions with parameters, return values, and state management

## Resources

Read more about [actions](/docs/actors/actions), [state](/docs/actors/state), and [actor setup](/docs/setup).

## License

MIT

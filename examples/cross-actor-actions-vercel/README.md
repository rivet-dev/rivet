> **Note:** This is the Vercel-optimized version of the [cross-actor-actions](../cross-actor-actions) example.
> It uses the `hono/vercel` adapter and is configured for Vercel deployment.

[![Deploy with Vercel](https://vercel.com/button)](https://vercel.com/new/clone?repository-url=https%3A%2F%2Fgithub.com%2Frivet-gg%2Frivet%2Ftree%2Fmain%2Fexamples%2Fcross-actor-actions-vercel&project-name=cross-actor-actions-vercel)

# Cross-Actor Actions

Demonstrates how actors can call actions on other actors for inter-actor communication and coordination.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/cross-actor-actions
npm install
npm run dev
```


## Features

- **Inter-actor communication**: Actors call actions on other actors using the server-side client
- **Transactional workflows**: Implement checkout process with inventory reservations
- **Distributed state management**: Each actor manages its own state while coordinating with others
- **Type-safe cross-actor calls**: Full TypeScript type safety across actor boundaries

## Implementation

This example demonstrates advanced inter-actor communication patterns:

- **Actor Definitions** ([`src/backend/registry.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/cross-actor-actions/src/backend/registry.ts)): Defines multiple actors (`cart`, `inventory`) that communicate with each other to implement a checkout workflow

## Resources

Read more about [communicating between actors](/docs/actors/communicating-between-actors), [actions](/docs/actors/actions), and [state](/docs/actors/state).

## License

MIT

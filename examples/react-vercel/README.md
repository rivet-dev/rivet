> **Note:** This is the Vercel-optimized version of the [react](../react) example.
> It uses the `hono/vercel` adapter and is configured for Vercel deployment.

[![Deploy with Vercel](https://vercel.com/button)](https://vercel.com/new/clone?repository-url=https%3A%2F%2Fgithub.com%2Frivet-gg%2Frivet%2Ftree%2Fmain%2Fexamples%2Freact-vercel&project-name=react-vercel)

# React Integration

Demonstrates React frontend integration with Rivet Actors.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/react
npm install
npm run dev
```


## Features

- **React frontend**: Build interactive UIs with React that connect to Rivet Actors
- **Type-safe client**: Use `@rivetkit/react` hooks for type-safe actor communication
- **Real-time updates**: Subscribe to actor events for live UI updates
- **Actor state management**: Actors handle backend logic while React manages UI state

## Implementation

This example demonstrates React frontend integration with Rivet Actors:

- **Actor Definition** ([`src/backend/registry.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/react/src/backend/registry.ts)): Backend actors with React frontend integration using type-safe hooks

## Resources

Read more about [React integration](/docs/platforms/react), [actions](/docs/actors/actions), [state](/docs/actors/state), and [events](/docs/actors/events).

## License

MIT

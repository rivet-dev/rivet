# React Query Integration

Demonstrates React Query integration with Rivet Actors.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/react-query
npm install
npm run dev
```


## Features

- **React frontend**: Build interactive UIs with React that connect to Rivet Actors
- **Type-safe client**: Use `@rivetkit/react` hooks for type-safe actor communication
- **Real-time updates**: Subscribe to actor events for live UI updates
- **Actor state management**: Actors handle backend logic while React manages UI state
- **React Query sync**: Mirror actor state into TanStack React Query for cached reads

## Implementation

This example demonstrates React Query integration with Rivet Actors:

- **Actor Definition** ([`src/actors.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/react-query/src/actors.ts)): Backend actors with React frontend integration using type-safe hooks
- **React Query cache** ([`frontend/App.tsx`](https://github.com/rivet-dev/rivet/tree/main/examples/react-query/frontend/App.tsx)): Keeps actor state mirrored in React Query

## Resources

Read more about [React integration](/docs/platforms/react), [actions](/docs/actors/actions), [state](/docs/actors/state), and [events](/docs/actors/events).

## License

MIT

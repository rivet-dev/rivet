# State Management

Demonstrates persistent state management in Rivet Actors with automatic state saving and restoration.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/state
npm install
npm run dev
```


## Features

- **Persistent state**: Actor state automatically saved and restored across restarts
- **Typed state management**: Full TypeScript type safety for state objects
- **State initialization**: Define initial state with `createState` or `state` property
- **Automatic serialization**: State changes automatically persisted without manual saves

## Implementation

This example demonstrates state management in Rivet Actors with a simple counter:

- **Actor Definition** ([`src/backend/registry.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/state/src/backend/registry.ts)): Defines the `counter` actor with a count state that persists across actor restarts

## Resources

Read more about [state management](/docs/actors/state), [actions](/docs/actors/actions), and [lifecycle hooks](/docs/actors/lifecycle).

## License

MIT

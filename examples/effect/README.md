# Effect Integration

Demonstrates how to integrate [Effect](https://effect.website/) with actors for functional, type-safe programming with powerful error handling and dependency injection.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/effect
npm install
npm run dev
```


## Features

- **Effect-wrapped actions** - Write actor actions using Effect generators for composable, type-safe logic
- **Actor context as Effect service** - Access actor state, broadcast, and other context via Effect's dependency injection
- **Structured logging** - Effect-based logging utilities integrated with actor logging

## Implementation

This example provides Effect bindings for actors. The core implementation wraps the actor context in Effect services, allowing you to write actions using Effect's generator syntax.

Key files:
- [`src/actors/fetch-actor.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/effect/src/actors/fetch-actor.ts) - Multi-step workflows with error handling
- [`src/actors/queue-processor.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/effect/src/actors/queue-processor.ts) - Background queue processing with the `run` handler

Example usage:

```typescript
import { actor } from "rivetkit";
import { Action } from "@rivetkit/effect";

export const counter = actor({
  state: { count: 0 },
  actions: {
    increment: Action.effect(function* (c, x: number) {
      yield* Action.updateState(c, (s) => { s.count += x; });
      const s = yield* Action.state(c);
      yield* Action.broadcast(c, "newCount", s.count);
      return s.count;
    }),
  },
});
```

## Resources

- [Effect Documentation](https://effect.website/docs/introduction)
- [Actions](/docs/actors/actions)
- [State](/docs/actors/state)

## License

MIT

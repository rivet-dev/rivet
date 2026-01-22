# Effect Integration for RivetKit

Demonstrates how to integrate [Effect](https://effect.website/) with RivetKit actors for functional, type-safe programming with powerful error handling and dependency injection.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet
cd rivet/examples/effect
npm install
npm run dev
```

## Features

- **Effect-wrapped actions** - Write actor actions using Effect generators for composable, type-safe logic
- **Durable workflows** - Use `@effect/workflow` with RivetKit's `waitUntil` for reliable multi-step operations
- **Actor context as Effect service** - Access actor state, broadcast, and other context via Effect's dependency injection
- **Structured logging** - Effect-based logging utilities integrated with RivetKit's actor logging

## Implementation

This example provides Effect bindings for RivetKit actors. The core implementation wraps RivetKit's actor context in Effect services, allowing you to write actions using Effect's generator syntax.

Key files:
- [`src/effect/action.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/effect/src/effect/action.ts) - Effect wrappers for action handlers with `Action.effect()` and `Action.workflow()`
- [`src/effect/actor.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/effect/src/effect/actor.ts) - Effect-wrapped actor context methods (state, broadcast, etc.)
- [`src/actors.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/effect/src/actors.ts) - Example actors using the Effect integration

Example usage:

```typescript
import { actor } from "rivetkit";
import { Action } from "./effect/index.ts";

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
- [RivetKit Actions](/docs/actors/actions)
- [RivetKit State](/docs/actors/state)

## License

Apache 2.0

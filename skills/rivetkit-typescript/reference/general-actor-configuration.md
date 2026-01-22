# Actor Configuration

> Source: `src/content/docs/general/actor-configuration.mdx`
> Canonical URL: https://rivet.gg/docs/general/actor-configuration
> Description: This page documents the configuration options available when defining a RivetKit actor. The actor configuration is passed to the `actor()` function.

---
## Basic Example

```typescript
import { actor, setup } from "rivetkit";

const myActor = actor({
  state: { count: 0 },

  actions: {
    increment: (c) => {
      c.state.count++;
      return c.state.count;
    },
  },
  options: {
    actionTimeout: 15_000,
  }
});

const registry = setup({
  use: { myActor },
});
```

## Configuration Reference

## Related

- [Registry Configuration](/docs/general/registry-configuration): Configure the RivetKit registry
- [State](/docs/actors/state): Managing actor state
- [Actions](/docs/actors/actions): Defining actor actions
- [Lifecycle](/docs/actors/lifecycle): Actor lifecycle hooks

_Source doc path: /docs/general/actor-configuration_

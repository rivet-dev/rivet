# Rivet Actor Client

Rivet Actors have built-in RPC, state, and events — the easiest way to build modern applications.

## Getting Started

- [Setup Guide](https://rivet.gg/docs/setup)
- [Documentation](https://rivet.gg/docs)
- [Examples](https://github.com/rivet-gg/rivet/tree/main/examples)

## Example

```typescript
import { Client } from "@rivet-gg/actor-client";
import type Counter from "./counter.ts";

const client = new Client(/* CONNECTION ADDRESS */);

// Get-or-create a counter actor
const counter = await client.get<Counter>({ name: "counter" });

// Listen for update count events (https://rivet.gg/docs/events)
counter.on("count", (count: number) => console.log("New count:", count));

// Increment the count over remote procedure call (https://rivet.gg/docs/rpc)
await counter.increment(1);

// Disconnect from the actor when finished (https://rivet.gg/docs/connections)
await counter.dispose();
```

See [setup guide](https://rivet.gg/docs/setup) for how to access `/* CONNECTION ADDRESS */`.

## Related Packages

- [Actor SDK (@rivet-gg/actor)](https://jsr.io/@rivet-gg/actor)

## Community & Support

- Join our [Discord](https://rivet.gg/discord)
- Follow us on [X](https://x.com/rivet_gg)
- Follow us on [Bluesky](https://bsky.app/profile/rivet-gg.bsky.social)
- File bug reports in [GitHub Issues](https://github.com/rivet-gg/rivet/issues)
- Post questions & ideas in [GitHub Discussions](https://github.com/orgs/rivet-gg/discussions)

## License

Apache 2.0


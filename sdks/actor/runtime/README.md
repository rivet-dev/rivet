# Rivet Actor

Rivet Actors have built-in RPC, state, and events — the easiest way to build modern applications.

## Getting Started

- [Setup Guide](https://rivet.gg/docs/setup)
- [Documentation](https://rivet.gg/docs)
- [Examples](https://github.com/rivet-gg/rivet/tree/main/examples)

## Usage

Make sure you've [installed the Rivet CLI](https://rivet.gg/docs/setup).

```sh
# Create project
rivet init

# Deploy actor
rivet deploy
```

See the [setup guide](https://rivet.gg/docs/setup) for more information.

## Example

```typescript
import { Actor } from "@rivet-gg/actor";
import type { Rpc } from "@rivet-gg/actor";

// Durable state for the counter (https://rivet.gg/docs/state)
interface State {
	count: number;
}

export default class Counter extends Actor<State> {
	// Create the initial state when the actor is first created (https://rivet.gg/docs/state)
	override _onInitialize(): State {
		return { count: 0 };
	}

	// Listen for state changes (https://rivet.gg/docs/lifecycle)
	override _onStateChange(newState: State): void | Promise<void> {
		// Broadcast a state update event to all clients (https://rivet.gg/docs/events)
		this._broadcast("count", newState.count);
	}

	// Expose a remote procedure call for clients to update the count (https://rivet.gg/docs/rpc)
	increment(_rpc: Rpc<Counter>, count: number): number {
		this._state.count += count;
		return this._state.count;
	}
}
```

## Related Packages

- [Actor Client SDK (@rivet-gg/actor-client)](https://jsr.io/@rivet-gg/actor-client)

## Community & Support

- Join our [Discord](https://rivet.gg/discord)
- Follow us on [X](https://x.com/rivet_gg)
- Follow us on [Bluesky](https://bsky.app/profile/rivet-gg.bsky.social)
- File bug reports in [GitHub Issues](https://github.com/rivet-gg/rivet/issues)
- Post questions & ideas in [GitHub Discussions](https://github.com/orgs/rivet-gg/discussions)

## License

Apache 2.0


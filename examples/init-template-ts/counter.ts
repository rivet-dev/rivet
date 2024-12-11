import { Actor } from "@rivet-gg/actors";
import type { Rpc } from "@rivet-gg/actors";

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
		this.state.count += count;
		return this.state.count;
	}
}

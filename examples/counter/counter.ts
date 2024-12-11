import { Actor } from "@rivet-gg/actors";
import type { Rpc, OnBeforeConnectOpts } from "@rivet-gg/actors";

interface State {
	count: number;
}

interface ConnState {
	mod: number;
}

interface ConnParams {
	mod: number;
}

export default class Counter extends Actor<State, ConnParams | undefined, ConnState> {
	override _onInitialize(): State {
		return { count: 0 };
	}

	override _onBeforeConnect(opts: OnBeforeConnectOpts<ConnParams>): ConnState {
		console.log("parameters", opts.parameters);
		return { mod: opts.parameters?.mod ?? 1 };
	}

	override _onStateChange(newState: State): void | Promise<void> {
		this._broadcast("broadcastCount", newState.count);

		for (const conn of this.connections.values()) {
			console.log("checking mod", conn.id, conn.state);
			console.log("state", conn.state);
			if (newState.count % conn.state.mod === 0) {
				conn.send("directCount", newState.count);
			}
		}
	}

	increment(_rpc: Rpc<Counter>, count: number): number {
		console.log(new Error("foo"));
		this.state.count += count;
		return this.state.count;
	}

	destroyMe() {
		this._shutdown();
	}
}

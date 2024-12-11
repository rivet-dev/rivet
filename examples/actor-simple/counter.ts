import { Actor, Rpc, Connection, OnBeforeConnectOpts } from "../../sdks/actors/runtime/src/mod.ts";

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
	//override _onConnect(opts: PrepareConnectionOpts<ConnParams>): ConnState {
	//	console.log('parameters', opts.parameters);
	//	return { mod: opts.parameters?.mod ?? 1 };
	//}

	override _onInitialize(): State {
		return { count: 0 };
	}

	override _onConnect(opts: OnBeforeConnectOpts<ConnParams>): ConnState {
		//if (!await authenticate(opts.params.token)) throw new Error("unauthenticated");
		console.log("parameters", opts.parameters);
		return { mod: opts.parameters?.mod ?? 1 };
	}

	override _onConnectionReady(_conn: Connection<this>) {
		// ...
	}

	override _onStateChange(newState: State): void | Promise<void> {
		this._broadcast("broadcastCount", newState.count);

		for (const conn of this.connections.values()) {
			console.log("checking mod", conn.id, conn.state);
			console.log("state", conn.state);
			if (newState.count % conn.state.mod == 0) {
				conn.send("directCount", newState.count);
			}
		}
	}

	increment(_rpc: Rpc<Counter>, count: number): number {
		this.state.count += count;
		return this.state.count;
	}

	destroyMe() {
		this._shutdown();
	}
}

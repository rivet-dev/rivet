import { Actor } from "@rivet-gg/actors";
import type { Rpc, OnBeforeConnectOptions } from "@rivet-gg/actors";

interface State {
	count: number;
}

interface ConnState {
	mod: number;
}

interface ConnParams {
	mod: number;
}

export default class Counter extends Actor<
	State,
	ConnParams | undefined,
	ConnState
> {
	override _onInitialize(): State {
		return { count: 0 };
	}

	override _onBeforeConnect(opts: OnBeforeConnectOptions<ConnParams>): ConnState {
		this._log.info("parameters", { params: opts.parameters });
		return { mod: opts.parameters?.mod ?? 1 };
	}

	override _onStateChange(newState: State): void | Promise<void> {
		this._broadcast("broadcastCount", newState.count);

		for (const conn of this._connections.values()) {
			this._log.info("checking mod", { id: conn.id, state: conn.state });
			this._log.info("state", { state: conn.state });
			if (newState.count % conn.state.mod === 0) {
				conn.send("directCount", newState.count);
			}
		}
	}

	increment(_rpc: Rpc<Counter>, count: number): number {
		this._log.info("increment", { count });
		this._state.count += count;
		return this._state.count;
	}

	destroyMe() {
		this._shutdown();
	}
}

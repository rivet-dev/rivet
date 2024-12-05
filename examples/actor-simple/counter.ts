import { PrepareConnectionOpts } from "../../sdks/actors/runtime/src/actor.ts";
import { Actor, Context } from "../../sdks/actors/runtime/src/mod.ts";

interface State {
    count: number;
}

interface ConnState {
	mod: number;
}

interface ConnParams {
	mod: number;
}

export default class Counter extends Actor<State, ConnParams, ConnState>  {
	protected override _prepareConnection(opts: PrepareConnectionOpts<ConnParams>): ConnState {
		return { mod: opts.parameters.mod ?? 1 };
	}

	protected override _onStateChange(newState: State): void | Promise<void> {
		this._broadcast("broadcastCount", newState.count);

		for (const conn of this.connections.values()) {
			console.log('checking mod', conn.id, conn.state);
			if (newState.count % conn.state!.mod == 0) {
				conn.send("directCount", newState.count);
			}
		}
	}

    override _createState(): State {
        return { count: 0 };
    }

    increment(c: Context<Counter>, count: number): number {
        this.state.count += count;
        return this.state.count;
    }

	destroyMe() {
		// HACK: Timeout to allow clean disconnect
		setTimeout(() => Deno.exit(0), 500);
	}
}


import { Connection } from "../../sdks/actors/runtime/src/connection.ts";
import Actor from "../../sdks/actors/runtime/src/mod.ts";

interface State {
    count: number;
}

interface ConnectionData {
	mod: number;
}

interface ConnectionParameters {
	mod: number;
}

export class Counter extends Actor<State, ConnectionData, ConnectionParameters>  {
	protected override onConnect(_conn: Connection<ConnectionData, ConnectionParameters>, parameters?: ConnectionParameters): ConnectionData {
		return { mod: parameters?.mod ?? 1 };
	}

	protected override onStateChange(newState: State): void | Promise<void> {
		this.broadcast("broadcastCount", newState.count);

		for (const conn of this.connections.values()) {
			console.log('checking mod', conn.id, conn.data);
			if (newState.count % conn.data!.mod == 0) {
				conn.send("directCount", newState.count);
			}
		}
	}

    override initializeState(): State {
        return { count: 0 };
    }

    increment(count: number): number {
        this.state.count += count;
        return this.state.count;
    }

	destroyMe() {
		// HACK: Timeout to allow clean disconnect
		setTimeout(() => Deno.exit(0), 500);
	}
}

// TODO: Clean up this syntax
new Counter().run();


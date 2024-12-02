import { Connection } from "../../sdks/actors/runtime/src/connection.ts";
import Actor from "../../sdks/actors/runtime/src/mod.ts";

interface State {
    count: number;
}

interface ConnectionData {
	mod: number;
}

class Counter extends Actor<State, ConnectionData>  {
	protected override onConnect(_conn: Connection<ConnectionData>, mod: number): ConnectionData {
		return { mod };
	}

	protected override onStateChange(newState: State): void | Promise<void> {
		this.broadcast("broadcastCount", newState.count);

		for (const conn of this.connections.values()) {
			if (newState.count % conn.data!.mod == 0) {
				conn.send("directCount", newState.count);
			}
		}
	}

    override initializeState(): State {
        return { count: 0 };
    }

    increment(): number {
        this.state.count += 1;
        return this.state.count;
    }

	destroyMe() {
		// TODO: Use the destroy API endpoint instead
		Deno.exit(0);
	}
}

// TODO: Clean up this syntax
new Counter().run();


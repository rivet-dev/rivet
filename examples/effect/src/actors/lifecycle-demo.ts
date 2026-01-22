import { actor } from "rivetkit";
import {
	Action,
	Log,
	OnCreate,
	OnWake,
	OnDestroy,
	OnSleep,
	OnConnect,
	OnDisconnect,
	OnStateChange,
} from "../effect/index.ts";

// Lifecycle demo actor - demonstrates all lifecycle hooks with Effect wrappers
interface LifecycleState {
	events: string[];
	connectionCount: number;
	lastStateChange: number;
}

export const lifecycleDemo = actor({
	state: {
		events: [],
		connectionCount: 0,
		lastStateChange: 0,
	} as LifecycleState,

	onCreate: OnCreate.effect(function* (c, input) {
		yield* Log.info("Actor created");
		c.state.events.push("onCreate");
	}),

	onWake: OnWake.effect(function* (c) {
		yield* Log.info("Actor woke up");
		c.state.events.push("onWake");
	}),

	onDestroy: OnDestroy.effect(function* (c) {
		yield* Log.info("Actor destroying");
		c.state.events.push("onDestroy");
	}),

	onSleep: OnSleep.effect(function* (c) {
		yield* Log.info("Actor going to sleep");
		c.state.events.push("onSleep");
	}),

	onStateChange: OnStateChange.effect(function* (c, newState) {
		// Note: OnStateChange is synchronous, so only use sync effects here
		c.state.lastStateChange = Date.now();
	}),

	onConnect: OnConnect.effect(function* (c, conn) {
		yield* Log.info("Client connected");
		c.state.connectionCount++;
		c.state.events.push("onConnect");
		yield* Action.broadcast(c, "userJoined", { connId: conn.id });
	}),

	onDisconnect: OnDisconnect.effect(function* (c, conn) {
		yield* Log.info("Client disconnected");
		c.state.connectionCount--;
		c.state.events.push("onDisconnect");
		yield* Action.broadcast(c, "userLeft", { connId: conn.id });
	}),

	actions: {
		getEvents: Action.effect(function* (c) {
			const s = yield* Action.state(c);
			return s.events;
		}),

		getConnectionCount: Action.effect(function* (c) {
			const s = yield* Action.state(c);
			return s.connectionCount;
		}),

		clearEvents: Action.effect(function* (c) {
			yield* Action.updateState(c, (s) => {
				s.events = [];
			});
			yield* Log.info("Events cleared");
		}),
	},
});

import { actor, event, setup } from "rivetkit";
import type { ActorConn } from "rivetkit/client";
import { createClient, createRivetKit, createRivetKitWithClient } from "./mod";

type Assert<T extends true> = T;
type IsEqual<A, B> =
	(<T>() => T extends A ? 1 : 2) extends <T>() => T extends B ? 1 : 2
		? true
		: false;

const counterActor = actor({
	state: {},
	events: {
		updated: event<{ count: number }>(),
		pair: event<[number, string]>(),
	},
	actions: {
		increment: (c, amount: number) => {
			c.broadcast("updated", { count: amount });
			return amount;
		},
	},
});

const registry = setup({
	use: {
		counter: counterActor,
	},
});

const client = createClient<typeof registry>();
const rivet = createRivetKitWithClient(client);
const rivetFromFactory = createRivetKit<typeof registry>();
const actorState = rivet.useActor({
	name: "counter",
	key: ["typecheck"],
});
const actorStateFromFactory = rivetFromFactory.useActor({
	name: "counter",
	key: ["typecheck-factory"],
});

if (actorState.connection) {
	void actorState.connection.increment(1);
	// @ts-expect-error action args should be typed
	void actorState.connection.increment("bad");
}

actorState.useEvent("updated", (payload) => {
	const count: number = payload.count;
	void count;
});

actorState.useEvent("pair", (count, label) => {
	const typedCount: number = count;
	const typedLabel: string = label;
	void typedCount;
	void typedLabel;
});
actorStateFromFactory.useEvent("updated", (payload) => {
	const count: number = payload.count;
	void count;
});

// @ts-expect-error unknown event name should fail
actorState.useEvent("missing", () => {});
// @ts-expect-error callback payload should be typed
actorState.useEvent("updated", (payload: { count: string }) => {
	void payload;
});

type ActualConnection = typeof actorState.connection;
type ExpectedConnection = ActorConn<typeof counterActor> | null;
const connectionTypeCheck: Assert<
	IsEqual<ActualConnection, ExpectedConnection>
> = true;
void connectionTypeCheck;

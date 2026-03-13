import { QueryClient } from "@tanstack/react-query";
import { actor, event, setup } from "rivetkit";
import type { ActorConn } from "rivetkit/client";
import {
	createActorQueryKey,
	createClient,
	createRivetKit,
	createRivetKitWithClient,
} from "./mod";

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
const queryClient = new QueryClient();
const rivetWithQuery = createRivetKitWithClient(client, {
	queryClient,
});

// biome-ignore lint/correctness/useHookAtTopLevel: hook usage is for typecheck coverage
const actorState = rivet.useActor({
	name: "counter",
	key: ["typecheck"],
});
// biome-ignore lint/correctness/useHookAtTopLevel: hook usage is for typecheck coverage
const actorStateFromFactory = rivetFromFactory.useActor({
	name: "counter",
	key: ["typecheck-factory"],
});
const actorQueryKey = createActorQueryKey({
	name: "counter",
	key: ["typecheck-query"],
});
// biome-ignore lint/correctness/useHookAtTopLevel: hook usage is for typecheck coverage
const actorQueryResult = rivetWithQuery.useActorQuery({
	name: "counter",
	key: ["typecheck-query"],
});
void actorQueryKey;
void actorQueryResult;

if (actorState.connection) {
	void actorState.connection.increment(1);
	// @ts-expect-error action args should be typed
	void actorState.connection.increment("bad");
}

// biome-ignore lint/correctness/useHookAtTopLevel: hook usage is for typecheck coverage
actorState.useEvent("updated", (payload) => {
	const count: number = payload.count;
	void count;
});

// biome-ignore lint/correctness/useHookAtTopLevel: hook usage is for typecheck coverage
actorState.useEvent("pair", (count, label) => {
	const typedCount: number = count;
	const typedLabel: string = label;
	void typedCount;
	void typedLabel;
});
// biome-ignore lint/correctness/useHookAtTopLevel: hook usage is for typecheck coverage
actorStateFromFactory.useEvent("updated", (payload) => {
	const count: number = payload.count;
	void count;
});

// @ts-expect-error unknown event name should fail
// biome-ignore lint/correctness/useHookAtTopLevel: hook usage is for typecheck coverage
actorState.useEvent("missing", () => { });
// @ts-expect-error callback payload should be typed
// biome-ignore lint/correctness/useHookAtTopLevel: hook usage is for typecheck coverage
actorState.useEvent("updated", (payload: { count: string }) => {
	void payload;
});

type ActualConnection = typeof actorState.connection;
type ExpectedConnection = ActorConn<typeof counterActor> | null;
const connectionTypeCheck: Assert<
	IsEqual<ActualConnection, ExpectedConnection>
> = true;
void connectionTypeCheck;

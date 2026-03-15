import { type ActorConnState, createRivetKit } from "@rivetkit/react";
import { useQuery } from "@tanstack/react-query";
import { useMemo } from "react";
import type { registry } from "../src/actors.ts";
import { queryClient } from "./query.ts";

// Step 1: Create the RivetKit client once at module level, passing your
// TanStack QueryClient so that actor connection state is automatically
// synced into the query cache under a single key per actor.
const { useActor, useActorQuery, createActorQueryKey } = createRivetKit<
	typeof registry
>(`${window.location.origin}/api/rivet`, { queryClient });

// The cache entry for each actor holds connection metadata (connStatus etc.)
// plus an optional `state` field you populate from events. One key covers both.
// Key shape: ["rivetkit", "actor", <name>, <key>, <params|null>, <noCreate>]
type CounterCacheEntry = ActorConnState<typeof registry, "counter"> & {
	state?: { count: number };
};

function App() {
	// Step 2: useActor connects to the actor. When a queryClient is passed to
	// createRivetKit, connection metadata is automatically synced into the cache.
	const counter = useActor({
		name: "counter",
		key: ["test-counter"],
	});

	const counterQueryKey = useMemo(
		() => createActorQueryKey({ name: "counter", key: ["test-counter"] }),
		[],
	);

	// Step 3: Subscribe to events and merge business state into the same cache
	// entry under a `state` field. Using the updater form of setQueryData
	// preserves the existing connection metadata.
	counter.useEvent("newCount", (newCount: number) => {
		queryClient.setQueryData<CounterCacheEntry>(counterQueryKey, (prev) => ({
			...prev,
			state: { count: newCount },
		} as CounterCacheEntry));
	});

	// Option A: Read the merged entry directly from the cache using the same key.
	// Useful in components that don't call useActor — as long as useActor is
	// mounted somewhere in the tree to populate the entry.
	const counterQuery = useQuery<CounterCacheEntry>({
		queryKey: counterQueryKey,
		queryFn: () => Promise.resolve(null as unknown as CounterCacheEntry),
		enabled: false,
	});

	// Option B: useActorQuery mounts the actor and returns a UseQueryResult in
	// one hook. Pass the merged type as a generic to access `state`.
	const counterHookQuery = useActorQuery<"counter", CounterCacheEntry>({
		name: "counter",
		key: ["test-counter"],
	});

	const increment = async () => {
		await counter.connection?.increment(1);
	};

	return (
		<div>
			{/* Count from event, merged into the actor's cache entry */}
			<h1>Counter: {counterQuery.data?.state?.count ?? "-"}</h1>

			{/* Same data via useActorQuery */}
			<p>Count (useActorQuery): {counterHookQuery.data?.state?.count ?? "-"}</p>

			{/* Connection metadata lives on the same entry */}
			<p>Connection status: {counterQuery.data?.connStatus ?? "-"}</p>

			<button type="button" onClick={increment}>
				Increment
			</button>
		</div>
	);
}

export default App;

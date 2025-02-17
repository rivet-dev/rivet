import type { Query, QueryClient } from "@tanstack/react-query";
import { z } from "zod";
import { queryClient } from "./global";

const watchResponseFragment = z.object({
	watch: z.object({ index: z.string() }),
});

async function watch(query: Query) {
	try {
		await query.promise;
	} catch (error) {
		// the query failed/cancelled, we should stop watching
		return;
	}
	const watchQueryState = queryClient.getQueryState([
		...query.queryKey,
		"watch",
	]);

	if (watchQueryState?.fetchStatus === "fetching") {
		// it means the watch query is already being fetched, so we don't need to watch it again
		return;
	}

	while (true) {
		await query.promise;
		const watchOptsParseResult = watchResponseFragment.safeParse(
			query.state.data,
		);

		if (!watchOptsParseResult.success) {
			// last query didn't have watch options, we should stop watching
			break;
		}

		const watchOpts = watchOptsParseResult.data;

		try {
			const result = await queryClient.fetchQuery({
				...query.options,
				retry: 0,
				gcTime: 0,
				staleTime: 0,
				queryKey: [...query.queryKey, "watch"],
				queryHash: JSON.stringify([...query.queryKey, "watch"]),
				meta: { __watcher: { index: watchOpts.watch.index } },
			});

			if (!result) {
				break;
			}

			// update the query with the new data
			queryClient.setQueryData(
				query.queryKey,
				typeof query.meta?.watch === "function"
					? query.meta.watch(query.state.data, result)
					: result,
			);
		} catch (error) {
			// something went wrong, we should stop watching
			// probably the query was cancelled
			break;
		}
	}
}

async function stopWatching(query: Query) {
	if (query.getObserversCount() <= 0) {
		const watchQuery = queryClient
			.getQueryCache()
			.find({ queryKey: [...query.queryKey, "watch"] });
		if (watchQuery) {
			watchQuery.cancel();
			watchQuery.destroy();
		}
	}
}

export function watchBlockingQueries(queryClient: QueryClient) {
	queryClient.getQueryCache().subscribe((event) => {
		if (event.type === "observerAdded") {
			if (event.query.meta?.watch) {
				watch(event.query);
			}
		}
		if (event.type === "observerRemoved") {
			stopWatching(event.query);
		}
	});
}

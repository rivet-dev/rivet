import {
	CatchBoundary,
	createFileRoute,
	redirect,
} from "@tanstack/react-router";
import { Actors } from "@/app/actors";

export const Route = createFileRoute("/_context/ns/$namespace/")({
	component: RouteComponent,
	beforeLoad: async ({ context }) => {
		if (context.__type !== "engine") {
			throw new Error("Invalid context type for this route");
		}
	},
	loaderDeps(opts) {
		return {
			n: opts.search.n,
		};
	},
	async loader({ context, deps, location }) {
		const dataProvider = context.dataProvider;
		const { actorId, actorKey } = location.search;

		// Prefetch runner configs so EmptyState doesn't flash "No Providers Connected"
		// while the queries are loading.
		const runnerPrefetch = Promise.all([
			context.queryClient.prefetchInfiniteQuery(
				dataProvider.runnerNamesQueryOptions(),
			),
			context.queryClient.prefetchInfiniteQuery(
				dataProvider.runnerConfigsQueryOptions(),
			),
		]);

		if (deps.n && (actorId || actorKey)) {
			await runnerPrefetch;
			return;
		}

		const builds = await context.queryClient.fetchInfiniteQuery(
			dataProvider.buildsQueryOptions(),
		);
		const firstBuildId = Object.keys(builds.pages[0]?.names ?? {})[0];

		if (!firstBuildId) {
			await runnerPrefetch;
			return;
		}

		const n = [firstBuildId];

		const [actors] = await Promise.all([
			context.queryClient.fetchInfiniteQuery(
				dataProvider.actorsListQueryOptions({ n }),
			),
			runnerPrefetch,
		]);
		const firstActorId = actors.pages[0]?.actors?.[0]?.key;

		if (!firstActorId) return;

		throw redirect({
			to: ".",
			search: (old) => ({
				...old,
				n,
				actorKey: firstActorId,
			}),
			replace: true,
		});
	},
});

export function RouteComponent() {
	const { actorKey, actorId } = Route.useSearch();

	return (
		<CatchBoundary
			getResetKey={() => actorKey ?? actorId ?? "no-actor-key"}
		>
			<Actors actorId={actorKey ?? actorId} />
		</CatchBoundary>
	);
}

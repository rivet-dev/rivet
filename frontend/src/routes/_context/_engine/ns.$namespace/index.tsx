import {
	CatchBoundary,
	createFileRoute,
	redirect,
} from "@tanstack/react-router";
import { Actors } from "@/app/actors";

export const Route = createFileRoute("/_context/_engine/ns/$namespace/")({
	component: RouteComponent,
	beforeLoad: async ({ context }) => {
		if (context.__type !== "engine") {
			throw new Error("Invalid context type for this route");
		}
	},
	loaderDeps(opts) {
		return {
			n: opts.search.n,
			actorId: opts.search.actorId,
		};
	},
	async loader({ context, deps }) {
		const dataProvider = context.dataProvider;

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

		if (deps.n || deps.actorId) {
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
		const firstActorId = actors.pages[0]?.actors?.[0]?.actorId;

		if (!firstActorId) return;

		throw redirect({
			to: ".",
			search: (old) => ({
				...old,
				n,
				actorId: firstActorId,
			}),
			replace: true,
		});
	},
});

export function RouteComponent() {
	const { actorId } = Route.useSearch();

	return (
		<CatchBoundary getResetKey={() => actorId ?? "no-actor-id"}>
			<Actors actorId={actorId} />
		</CatchBoundary>
	);
}

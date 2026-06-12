import {
	CatchBoundary,
	createFileRoute,
	redirect,
} from "@tanstack/react-router";
import { Actors } from "@/app/actors";
import { EngineNamespaceLanding } from "@/app/engine-namespace-landing";

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
		const { actorId } = location.search as Record<
			string,
			string | undefined
		>;

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

		if (deps.n && actorId) {
			await runnerPrefetch;
			return;
		}

		const n: string[] = deps.n || [];

		// Without a selected Actor name, render the namespace landing (the Actor
		// grid) instead of auto-redirecting into the first build's instances.
		// Matches the cloud route so OSS and platform behave the same.
		if (!n[0]) {
			await Promise.all([
				runnerPrefetch,
				context.queryClient.prefetchInfiniteQuery(
					dataProvider.buildsQueryOptions(),
				),
			]);
			return;
		}

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
	const { actorId, n } = Route.useSearch();

	return (
		<CatchBoundary getResetKey={() => actorId ?? "no-actor-id"}>
			<NamespaceActors
				actorId={actorId}
				hasSelection={!!(n && n.length > 0)}
			/>
		</CatchBoundary>
	);
}

// With no Actor name selected, show the full-page namespace landing (the Actor
// grid, which itself surfaces the no-providers / no-actors states). Selecting a
// build sets `n` and switches to the Actor list/detail. Mirrors the cloud route
// so OSS and platform behave the same.
function NamespaceActors({
	actorId,
	hasSelection,
}: {
	actorId: string | undefined;
	hasSelection: boolean;
}) {
	if (!hasSelection) {
		return <EngineNamespaceLanding />;
	}

	return <Actors actorId={actorId} />;
}

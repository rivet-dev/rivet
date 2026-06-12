import { useSuspenseQuery } from "@tanstack/react-query";
import {
	CatchBoundary,
	createFileRoute,
	redirect,
} from "@tanstack/react-router";
import { Actors } from "@/app/actors";
import { ActorsGrid } from "@/app/actors-grid";
import { useEngineNamespaceDataProvider } from "@/components/actors";

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

		// Without a selected actor name, render the grid landing instead of
		// auto-redirecting into the first build's instances.
		if (!n[0]) {
			await Promise.all([
				runnerPrefetch,
				context.queryClient.prefetchQuery(
					dataProvider.currentNamespaceQueryOptions(),
				),
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
	pendingComponent: PendingComponent,
});

export function RouteComponent() {
	const search = Route.useSearch() as Record<string, unknown>;
	const actorId = search.actorId as string | undefined;
	const actorKey = search.actorKey as string | undefined;
	const n = search.n as string[] | undefined;
	const hasSelection = !!(actorId || actorKey || n?.length);

	if (!hasSelection) {
		return <NamespaceLanding />;
	}

	const id = actorKey ?? actorId;
	return (
		<CatchBoundary getResetKey={() => id ?? "no-actor-id"}>
			<Actors actorId={id} />
		</CatchBoundary>
	);
}

function NamespaceLanding() {
	const dataProvider = useEngineNamespaceDataProvider();
	const { data: namespace } = useSuspenseQuery(
		dataProvider.currentNamespaceQueryOptions(),
	);

	return <ActorsGrid namespaceLabel={namespace?.displayName} />;
}

function PendingComponent() {
	const search = Route.useSearch() as Record<string, unknown>;
	const actorId = search.actorId as string | undefined;
	const actorKey = search.actorKey as string | undefined;
	const n = search.n as string[] | undefined;
	const hasSelection = !!(actorId || actorKey || n?.length);

	if (!hasSelection) {
		return <ActorsGrid.Skeleton />;
	}
	return null;
}

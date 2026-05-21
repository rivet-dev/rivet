import { useSuspenseQuery } from "@tanstack/react-query";
import { createFileRoute, notFound, redirect } from "@tanstack/react-router";
import { Actors } from "@/app/actors";
import { ActorsGrid } from "@/app/actors-grid";
import { useCloudNamespaceDataProvider } from "@/components/actors";

export const Route = createFileRoute(
	"/_context/orgs/$organization/projects/$project/ns/$namespace/",
)({
	component: RouteComponent,
	beforeLoad: async ({ context }) => {
		if (context.__type !== "cloud") {
			throw notFound();
		}
	},
	loaderDeps(opts) {
		return {
			n: opts.search.n,
		};
	},
	async loader({ context, deps, location }) {
		const dataProvider = context.dataProvider;
		const { actorId } = (location.search as Record<string, string>) || {};

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

	return (
		<NamespaceContent
			hasSelection={hasSelection}
			actorId={actorKey ?? actorId}
		/>
	);
}

function NamespaceContent({
	hasSelection,
	actorId,
}: {
	hasSelection: boolean;
	actorId: string | undefined;
}) {
	const { namespace: namespaceParam } = Route.useParams();
	const dataProvider = useCloudNamespaceDataProvider();
	const { data: namespace } = useSuspenseQuery(
		dataProvider.currentProjectNamespaceQueryOptions({
			namespace: namespaceParam,
		}),
	);

	if (!hasSelection) {
		return <ActorsGrid namespaceLabel={namespace.displayName} />;
	}
	return <Actors actorId={actorId} />;
}

function PendingComponent() {
	const search = Route.useSearch() as Record<string, unknown>;
	const actorId = search.actorId as string | undefined;
	const actorKey = search.actorKey as string | undefined;
	const n = search.n as string[] | undefined;
	const hasSelection = !!(actorId || actorKey || n?.length);

	return (
		<NamespaceContent
			hasSelection={hasSelection}
			actorId={actorKey ?? actorId}
		/>
	);
}

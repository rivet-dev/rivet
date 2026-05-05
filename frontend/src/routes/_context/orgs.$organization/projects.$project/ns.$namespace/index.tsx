import {
	CatchBoundary,
	createFileRoute,
	notFound,
	redirect,
} from "@tanstack/react-router";
import { Actors } from "@/app/actors";

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

		if (!n[0]) {
			const builds = await context.queryClient.fetchInfiniteQuery(
				dataProvider.buildsQueryOptions(),
			);
			const firstBuildId = Object.keys(builds.pages[0]?.names ?? {})[0];

			if (!firstBuildId) {
				await runnerPrefetch;
				return;
			}

			n[0] = firstBuildId;
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
	const { actorId } = Route.useSearch();

	return <Actors actorId={actorId} />;
}

function PendingComponent() {
	const { actorId } = Route.useSearch();

	return <Actors actorId={actorId} />;
}

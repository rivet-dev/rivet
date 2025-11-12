import type { QueryClient } from "@tanstack/react-query";
import {
	CatchBoundary,
	createFileRoute,
	notFound,
	redirect,
} from "@tanstack/react-router";
import { Actors } from "@/app/actors";
import { BuildPrefiller } from "@/app/build-prefiller";
import {
	useDataProvider,
	type useEngineCompatDataProvider,
} from "@/components/actors";

export const Route = createFileRoute(
	"/_context/_cloud/orgs/$organization/projects/$project/ns/$namespace/",
)({
	component: RouteComponent,
	beforeLoad: async ({ context }) => {
		if (context.__type !== "cloud") {
			throw notFound();
		}

		const shouldDisplay = await shouldDisplayActors(context);

		if (!shouldDisplay) {
			throw redirect({ from: Route.to, replace: true, to: "./connect" });
		}
	},
});

export async function shouldDisplayActors(context: {
	queryClient: QueryClient;
	dataProvider: ReturnType<typeof useEngineCompatDataProvider>;
}) {
	try {
		const infiniteBuilds = await context.queryClient.fetchInfiniteQuery(
			context.dataProvider.buildsQueryOptions(),
		);

		const hasNames = infiniteBuilds.pages.some(
			(page) => Object.keys(page.names).length > 0,
		);

		const infiniteRunnerConfigs =
			await context.queryClient.fetchInfiniteQuery(
				context.dataProvider.runnerConfigsQueryOptions(),
			);

		const hasRunnerConfigs = infiniteRunnerConfigs.pages.some(
			(page) => Object.keys(page.runnerConfigs).length > 0,
		);

		if (!hasNames && !hasRunnerConfigs) {
			return undefined;
		}

		return true;
	} catch {
		return undefined;
	}
}

export function RouteComponent() {
	const { actorId, n } = Route.useSearch();
	const provider = useDataProvider();

	// HACK: not sure why it happens
	if (!provider.buildsQueryOptions) {
		return null;
	}

	return (
		<CatchBoundary getResetKey={() => actorId ?? "no-actor-id"}>
			<Actors actorId={actorId} />
			<CatchBoundary
				getResetKey={() => n?.join(",") ?? "no-build-name"}
				errorComponent={() => null}
			>
				{!n ? <BuildPrefiller /> : null}
			</CatchBoundary>
		</CatchBoundary>
	);
}

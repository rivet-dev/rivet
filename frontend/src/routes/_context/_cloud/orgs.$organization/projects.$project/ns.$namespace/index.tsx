import {
	CatchBoundary,
	createFileRoute,
	type InferAllContext,
	notFound,
	redirect,
} from "@tanstack/react-router";
import { Actors } from "@/app/actors";
import { BuildPrefiller } from "@/app/build-prefiller";
import { useDataProvider } from "@/components/actors";

export const Route = createFileRoute(
	"/_context/_cloud/orgs/$organization/projects/$project/ns/$namespace/",
)({
	component: RouteComponent,
	beforeLoad: async ({ context }) => {
		if (context.__type !== "cloud") {
			throw notFound();
		}

		const isVisible = await shouldDisplayActors(context);

		if (!isVisible) {
			throw redirect({ from: Route.to, replace: true, to: "./connect" });
		}
	},
});

async function shouldDisplayActors(context: InferAllContext<typeof Route>) {
	try {
		const infiniteBuilds = await context.queryClient.fetchInfiniteQuery(
			context.dataProvider.buildsQueryOptions(),
		);

		const hasBuilds = infiniteBuilds.pages.some(
			(page) => page.builds.length > 0,
		);

		const infiniteRunnerConfigs =
			await context.queryClient.fetchInfiniteQuery(
				context.dataProvider.runnerConfigsQueryOptions(),
			);

		const hasRunnerConfigs = infiniteRunnerConfigs.pages.some(
			(page) => Object.keys(page.runnerConfigs).length > 0,
		);

		if (!hasBuilds && !hasRunnerConfigs) {
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
		<>
			<CatchBoundary getResetKey={() => actorId ?? "no-actor-id"}>
				<Actors actorId={actorId} />
				<CatchBoundary
					getResetKey={() => n?.join(",") ?? "no-build-name"}
					errorComponent={() => null}
				>
					{!n ? <BuildPrefiller /> : null}
				</CatchBoundary>
			</CatchBoundary>
		</>
	);
}

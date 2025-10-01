import {
	CatchBoundary,
	createFileRoute,
	type InferAllContext,
	notFound,
	RouteContext,
	redirect,
} from "@tanstack/react-router";
import { Actors } from "@/app/actors";
import { BuildPrefiller } from "@/app/build-prefiller";

export const Route = createFileRoute(
	"/_context/_cloud/orgs/$organization/projects/$project/ns/$namespace/",
)({
	component: RouteComponent,
	beforeLoad: async ({ context }) => {
		if (context.__type !== "cloud") {
			throw notFound();
		}

		const build = await getAnyBuild(context);

		if (!build) {
			throw redirect({ from: Route.to, replace: true, to: "./connect" });
		}
	},
});

async function getAnyBuild(context: InferAllContext<typeof Route>) {
	try {
		const result = await context.queryClient.fetchInfiniteQuery(
			context.dataProvider.buildsQueryOptions(),
		);

		return result.pages[0].builds[0];
	} catch {
		return undefined;
	}
}

export function RouteComponent() {
	const { actorId, n } = Route.useSearch();

	return (
		<>
			<CatchBoundary getResetKey={() => actorId ?? "no-actor-id"}>
				<Actors actorId={actorId} />
				{!n ? <BuildPrefiller /> : null}
			</CatchBoundary>
		</>
	);
}

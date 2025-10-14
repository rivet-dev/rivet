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
import { useDataProvider } from "@/components/actors";

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

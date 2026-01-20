import {
	CatchBoundary,
	createFileRoute,
	notFound,
} from "@tanstack/react-router";
import { Actors } from "@/app/actors";
import { BuildPrefiller } from "@/app/build-prefiller";
import { FullscreenLoading } from "@/components";

export const Route = createFileRoute(
	"/_context/_cloud/orgs/$organization/projects/$project/ns/$namespace/",
)({
	component: RouteComponent,
	beforeLoad: async ({ context }) => {
		if (context.__type !== "cloud") {
			throw notFound();
		}
	},
	pendingComponent: FullscreenLoading,
});

export function RouteComponent() {
	const { actorId, n } = Route.useSearch();

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

import { createFileRoute } from "@tanstack/react-router";
import { Actors } from "@/app/actors";
import { BuildPrefiller } from "@/app/build-prefiller";

export const Route = createFileRoute(
	"/_context/_cloud/orgs/$organization/projects/$project/ns/$namespace/",
)({
	component: RouteComponent,
});

export function RouteComponent() {
	const { actorId, n } = Route.useSearch();

	return (
		<>
			<Actors actorId={actorId} />
			{!n ? <BuildPrefiller /> : null}
		</>
	);
}

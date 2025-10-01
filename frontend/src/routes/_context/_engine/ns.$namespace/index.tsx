import { createFileRoute, useSearch } from "@tanstack/react-router";
import { Actors } from "@/app/actors";
import { BuildPrefiller } from "@/app/build-prefiller";

export const Route = createFileRoute("/_context/_engine/ns/$namespace/")({
	component: RouteComponent,
});

export function RouteComponent() {
	const { actorId, n } = useSearch({ from: "/_context" });

	return (
		<>
			<Actors actorId={actorId} />
			{!n ? <BuildPrefiller /> : null}
		</>
	);
}

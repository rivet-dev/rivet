import {
	CatchBoundary,
	createFileRoute,
	useSearch,
} from "@tanstack/react-router";
import { Actors } from "@/app/actors";
import { BuildPrefiller } from "@/app/build-prefiller";

export const Route = createFileRoute("/_context/_engine/ns/$namespace/")({
	component: RouteComponent,
	beforeLoad: async ({ context }) => {
		if (context.__type !== "engine") {
			throw new Error("Invalid context type for this route");
		}
	},
});

export function RouteComponent() {
	const { actorId, n } = useSearch({ from: "/_context" });

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

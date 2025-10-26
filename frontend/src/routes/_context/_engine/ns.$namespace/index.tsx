import {
	CatchBoundary,
	createFileRoute,
	redirect,
	useSearch,
} from "@tanstack/react-router";
import { Actors } from "@/app/actors";
import { BuildPrefiller } from "@/app/build-prefiller";
import { shouldDisplayActors } from "../../_cloud/orgs.$organization/projects.$project/ns.$namespace/index";

export const Route = createFileRoute("/_context/_engine/ns/$namespace/")({
	component: RouteComponent,
	beforeLoad: async ({ context }) => {
		if (context.__type !== "engine") {
			throw new Error("Invalid context type for this route");
		}

		const shouldDisplay = await shouldDisplayActors(context);

		if (!shouldDisplay) {
			throw redirect({ from: Route.to, replace: true, to: "./connect" });
		}
	},
});

export function RouteComponent() {
	const { actorId, n } = useSearch({ from: "/_context" });

	return (
		<>
			<Actors actorId={actorId} />

			<CatchBoundary
				getResetKey={() => n?.join(",") ?? "no-build-name"}
				errorComponent={() => null}
			>
				{!n ? <BuildPrefiller /> : null}
			</CatchBoundary>
		</>
	);
}

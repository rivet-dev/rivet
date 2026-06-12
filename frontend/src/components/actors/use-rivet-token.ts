import { useQuery } from "@tanstack/react-query";
import { useRouteContext } from "@tanstack/react-router";
import { features } from "@/lib/features";
import { useEngineCompatDataProvider } from "./data-provider";

// Returns the rivet token (cloud publishable token in cloud mode, engine
// admin token in self-hosted mode). Drives the `rivet_token` WebSocket
// subprotocol header that the inspector connection requires for auth.
//
// Token source per deployment:
//   - cloud: publishableTokenQueryOptions() from the route's cloud data
//     provider — has a ~5min cache window
//   - self-hosted: engineAdminTokenQueryOptions() from the engine data
//     provider — reads from localStorage
//
// Hook order branches on `features.platform`, which is a compile-time
// constant per build, so the conditional hook calls are static at runtime.
export function useRivetToken(): string {
	if (features.platform) {
		// biome-ignore lint/correctness/useHookAtTopLevel: guarded by build constant
		const { data } = useQuery(
			// biome-ignore lint/correctness/useHookAtTopLevel: guarded by build constant
			useRouteContext({
				from: "/_context/orgs/$organization/projects/$project/ns/$namespace",
			}).dataProvider.publishableTokenQueryOptions(),
		);
		return data || "";
	}
	// biome-ignore lint/correctness/useHookAtTopLevel: guarded by build constant
	const { data } = useQuery(
		// biome-ignore lint/correctness/useHookAtTopLevel: guarded by build constant
		useEngineCompatDataProvider().engineAdminTokenQueryOptions(),
	);
	return data || "";
}

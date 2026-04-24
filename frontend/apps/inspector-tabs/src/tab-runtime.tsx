import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { type ReactNode, useMemo } from "react";
import type { ActorId } from "@/components/actors/queries";
import { MockActorInspectorProvider } from "./mock-inspector-context";
import { TabContextProvider } from "./tab-context";

/**
 * Reads the actorId from the iframe's URL search params.
 * Called at module level in each tab entry's main.tsx.
 */
export function getActorIdFromUrl(): ActorId {
	const params = new URLSearchParams(window.location.search);
	return (params.get("actorId") ?? "") as ActorId;
}

/**
 * Root provider for all iframe tab bundles. Supplies a fresh QueryClient,
 * the postMessage bridge context, and the mock inspector context so that
 * existing tab components (ActorStateTab etc.) work without modification.
 */
export function TabRuntime({
	actorId,
	children,
}: {
	actorId: ActorId;
	children: ReactNode;
}) {
	const queryClient = useMemo(
		() =>
			new QueryClient({
				defaultOptions: {
					queries: {
						// Data arrives via postMessage / broadcastQueryClient —
						// never refetch automatically in the tab.
						staleTime: Number.POSITIVE_INFINITY,
						retry: false,
					},
				},
			}),
		[],
	);

	return (
		<QueryClientProvider client={queryClient}>
			<TabContextProvider actorId={actorId} queryClient={queryClient}>
				<MockActorInspectorProvider>
					{children}
				</MockActorInspectorProvider>
			</TabContextProvider>
		</QueryClientProvider>
	);
}

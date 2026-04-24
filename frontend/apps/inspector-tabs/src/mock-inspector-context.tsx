import { type ReactNode, useMemo } from "react";
import {
	ActorInspectorContext,
	createDefaultActorInspectorContext,
} from "@/components/actors/actor-inspector-context";
import { useTabContext } from "./tab-context";

/**
 * Provides a mock ActorInspectorContext for iframe tab bundles. All API calls
 * are routed through postMessage to the shell, which owns the real WebSocket
 * inspector connection.
 */
export function MockActorInspectorProvider({
	children,
}: {
	children: ReactNode;
}) {
	const { sendAction, features, rivetkitVersion, inspectorProtocolVersion } =
		useTabContext();

	const contextValue = useMemo(() => {
		const api = {
			ping: () => sendAction({ name: "ping", args: [] }) as Promise<void>,
			executeAction: (name: string, args: unknown[]) =>
				sendAction({ name: "executeAction", args: [name, args] }),
			patchState: (state: unknown) =>
				sendAction({ name: "patchState", args: [state] }) as Promise<void>,
			getConnections: () =>
				sendAction({ name: "getConnections", args: [] }) as ReturnType<
					typeof api.getConnections
				>,
			getState: () =>
				sendAction({ name: "getState", args: [] }) as ReturnType<
					typeof api.getState
				>,
			getRpcs: () =>
				sendAction({ name: "getRpcs", args: [] }) as ReturnType<
					typeof api.getRpcs
				>,
			getTraces: (options: Parameters<typeof api.getTraces>[0]) =>
				sendAction({ name: "getTraces", args: [options] }) as ReturnType<
					typeof api.getTraces
				>,
			getQueueStatus: (limit: number) =>
				sendAction({ name: "getQueueStatus", args: [limit] }) as ReturnType<
					typeof api.getQueueStatus
				>,
			getWorkflowHistory: () =>
				sendAction({
					name: "getWorkflowHistory",
					args: [],
				}) as ReturnType<typeof api.getWorkflowHistory>,
			replayWorkflowFromStep: (entryId?: string) =>
				sendAction({
					name: "replayWorkflowFromStep",
					args: [entryId],
				}) as ReturnType<typeof api.replayWorkflowFromStep>,
			getDatabaseSchema: () =>
				sendAction({ name: "getDatabaseSchema", args: [] }) as ReturnType<
					typeof api.getDatabaseSchema
				>,
			getDatabaseTableRows: (
				table: string,
				limit: number,
				offset: number,
			) =>
				sendAction({
					name: "getDatabaseTableRows",
					args: [table, limit, offset],
				}) as ReturnType<typeof api.getDatabaseTableRows>,
			executeDatabaseSql: (
				request: Parameters<typeof api.executeDatabaseSql>[0],
			) =>
				sendAction({
					name: "executeDatabaseSql",
					args: [request],
				}) as ReturnType<typeof api.executeDatabaseSql>,
			getMetadata: () =>
				sendAction({ name: "getMetadata", args: [] }) as ReturnType<
					typeof api.getMetadata
				>,
		};

		return {
			...createDefaultActorInspectorContext({ api }),
			// Tabs are always "connected" from their own perspective — they receive
			// data from the shell which manages the actual connection status.
			connectionStatus: "connected" as const,
			isInspectorAvailable: true,
			rivetkitVersion,
			inspectorProtocolVersion,
			features,
		};
	}, [sendAction, features, rivetkitVersion, inspectorProtocolVersion]);

	return (
		<ActorInspectorContext.Provider value={contextValue}>
			{children}
		</ActorInspectorContext.Provider>
	);
}

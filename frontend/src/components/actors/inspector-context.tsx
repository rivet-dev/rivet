import { mutationOptions, queryOptions } from "@tanstack/react-query";
import { createContext, useContext, useEffect, useMemo } from "react";
import { useWebSocket } from "../hooks/use-websocket";
import type { ActorId } from "./queries";

export const createDefaultActorInspectorContext = () => ({
	actorStateQueryOptions(actorId: ActorId) {
		return queryOptions({
			refetchInterval: 1000,
			queryKey: ["actor", actorId, "state"],
		});
	},

	actorConnectionsQueryOptions(actorId: ActorId) {
		return queryOptions({
			queryKey: ["actor", actorId, "connections"],
		});
	},

	actorDatabaseQueryOptions(actorId: ActorId) {
		return queryOptions({
			queryKey: ["actor", actorId, "database"],
		});
	},

	actorDatabaseEnabledQueryOptions(actorId: ActorId) {
		return queryOptions({
			...this.actorDatabaseQueryOptions(actorId),
			select: (data) => data.enabled,
		});
	},

	actorDatabaseTablesQueryOptions(actorId: ActorId) {
		return queryOptions({
			...this.actorDatabaseQueryOptions(actorId),
			select: (data) =>
				data.db?.map((table) => ({
					name: table.table.name,
					type: table.table.type,
					records: table.records,
				})) || [],
			notifyOnChangeProps: ["data", "isError", "isLoading"],
		});
	},

	actorDatabaseRowsQueryOptions(actorId: ActorId, table: string) {
		return queryOptions({
			queryKey: ["actor", actorId, "database", table],
		});
	},

	actorEventsQueryOptions(actorId: ActorId) {
		return queryOptions({
			queryKey: ["actor", actorId, "events"],
		});
	},

	actorRpcsQueryOptions(actorId: ActorId) {
		return queryOptions({
			queryKey: ["actor", actorId, "rpcs"],
		});
	},

	actorClearEventsMutationOptions(actorId: ActorId) {
		return mutationOptions({
			mutationKey: ["actor", actorId, "clear-events"],
			// TODO:
		});
	},

	actorWakeUpMutationOptions(actorId: ActorId) {
		return mutationOptions({
			mutationKey: ["actor", actorId, "wake-up"],
			// TODO:
		});
	},

	actorAutoWakeUpQueryOptions(
		actorId: ActorId,
		{ enabled }: { enabled?: boolean } = {},
	) {
		return queryOptions({
			enabled,
			refetchInterval: 1000,
			staleTime: 0,
			gcTime: 0,
			queryKey: ["actor", actorId, "auto-wake-up"],
			retry: false,
			//FIXME:
		});
	},
});

export type ActorInspectorContext = ReturnType<
	typeof createDefaultActorInspectorContext
>;

const ActorInspectorContext = createContext({} as ActorInspectorContext);

export const useActorInspector = () => useContext(ActorInspectorContext);

export const ActorInspectorProvider = ({
	children,
	value,
	actorId,
	credentials,
}: {
	children: React.ReactNode;
	value: ActorInspectorContext;
	actorId?: ActorId;
	credentials: { url: string; token: string };
}) => {
	const protocols = useMemo(
		() => [
			`rivet_target.actor`,
			`rivet_actor.${actorId}`,
			`rivet_encoding.bare`,
			`rivet_token.${credentials.token}`,
		],
		[actorId, credentials.token],
	);

	const f = useWebSocket(credentials.url, protocols);

	console.log(f);

	return (
		<ActorInspectorContext.Provider value={value}>
			{/* {children} */}
		</ActorInspectorContext.Provider>
	);
};

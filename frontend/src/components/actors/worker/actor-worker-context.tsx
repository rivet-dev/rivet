/** biome-ignore-all lint/correctness/useHookAtTopLevel: guarded by build constant */
import { useQuery } from "@tanstack/react-query";
import {
	createContext,
	type ReactNode,
	useCallback,
	useContext,
	useEffect,
	useState,
	useSyncExternalStore,
} from "react";
import { match } from "ts-pattern";
import { assertNonNullable } from "../../lib/utils";
import { useActorInspector } from "../actor-inspector-context";
import { useDataProvider, useEngineCompatDataProvider } from "../data-provider";
import { useActorInspectorData } from "../hooks/use-actor-inspector-data";
import type { ActorId } from "../queries";
import { ActorWorkerContainer } from "./actor-worker-container";

export const ActorWorkerContext = createContext<ActorWorkerContainer | null>(
	null,
);

export const useActorWorker = () => {
	const value = useContext(ActorWorkerContext);
	assertNonNullable(value);
	return value;
};

const useConnectionDetails = () => {
	return match(__APP_TYPE__)
		.with("inspector", () => {
			return { namespace: "", engineToken: "" };
		})
		.otherwise(() => {
			// biome-ignore lint/correctness/useHookAtTopLevel: guarded by build constant
			const provider = useEngineCompatDataProvider();
			return {
				namespace: provider.engineNamespace,
				engineToken: provider.engineToken,
			};
		});
};

interface ActorWorkerContextProviderProps {
	actorId: ActorId;
	children: ReactNode;
}
export const ActorWorkerContextProvider = ({
	children,
	actorId,
}: ActorWorkerContextProviderProps) => {
	const dataProvider = useDataProvider();
	const { engineToken, namespace } = useConnectionDetails();

	const {
		data: {
			name,
			endpoint,
			destroyedAt,
			startedAt,
			sleepingAt,
			runner,
		} = {},
	} = useQuery(dataProvider.actorWorkerQueryOptions(actorId));
	const { token: inspectorToken } = useActorInspectorData(actorId);

	const enabled = (!destroyedAt && !sleepingAt && !!startedAt) ?? false;

	const actorInspector = useActorInspector();
	const { data: rpcs = [] } = useQuery({
		...actorInspector.actorRpcsQueryOptions(actorId),
		enabled,
	});

	const [container] = useState<ActorWorkerContainer>(
		() => new ActorWorkerContainer(),
	);

	// biome-ignore lint/correctness/useExhaustiveDependencies: we want to create worker on each of those props change
	useEffect(() => {
		const ctrl = new AbortController();

		if (enabled) {
			container.init({
				actorId,
				endpoint,
				name,
				signal: ctrl.signal,
				rpcs: rpcs ?? [],
				engineToken,
				runnerName: runner,
				namespace,
				inspectorToken,
				invokeAction: actorInspector.api.executeAction,
			});
		}

		return () => {
			ctrl.abort();
			container.terminate();
		};
	}, [
		actorId,
		enabled,
		rpcs,
		name,
		endpoint,
		engineToken,
		inspectorToken,
		namespace,
		runner,
		actorInspector.api.executeAction,
	]);

	return (
		<ActorWorkerContext.Provider value={container}>
			{children}
		</ActorWorkerContext.Provider>
	);
};

export function useActorReplCommands() {
	const container = useActorWorker();
	return useSyncExternalStore(
		useCallback(
			(cb) => {
				return container.subscribe(cb);
			},
			[container],
		),
		useCallback(() => {
			return container.getCommands();
		}, [container]),
	);
}

export function useActorWorkerStatus() {
	const container = useActorWorker();
	return useSyncExternalStore(
		useCallback(
			(cb) => {
				return container.subscribe(cb);
			},
			[container],
		),
		useCallback(() => {
			return container.getStatus();
		}, [container]),
	);
}

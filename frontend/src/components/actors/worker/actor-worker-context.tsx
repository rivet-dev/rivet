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
import { assertNonNullable } from "../../lib/utils";
import { useActor } from "../actor-queries-context";
import { useManager } from "../manager-context";
import { ActorFeature, type ActorId } from "../queries";
import { ActorWorkerContainer } from "./actor-worker-container";

export const ActorWorkerContext = createContext<ActorWorkerContainer | null>(
	null,
);

export const useActorWorker = () => {
	const value = useContext(ActorWorkerContext);
	assertNonNullable(value);
	return value;
};

interface ActorWorkerContextProviderProps {
	actorId: ActorId;
	children: ReactNode;
}
export const ActorWorkerContextProvider = ({
	children,
	actorId,
}: ActorWorkerContextProviderProps) => {
	const { data: { features, name, endpoint, destroyedAt, startedAt } = {} } =
		useQuery(useManager().actorWorkerQueryOptions(actorId));
	const enabled =
		(features?.includes(ActorFeature.Console) &&
			!destroyedAt &&
			!!startedAt) ??
		false;

	const actorQueries = useActor();
	const { data: { rpcs } = {} } = useQuery(
		actorQueries.actorRpcsQueryOptions(actorId, { enabled }),
	);

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
				rpcs,
			});
		}

		return () => {
			ctrl.abort();
			container.terminate();
		};
	}, [actorId, enabled, rpcs, name, endpoint]);

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

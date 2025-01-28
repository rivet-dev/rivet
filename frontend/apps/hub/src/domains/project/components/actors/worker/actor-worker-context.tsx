import { assertNonNullable } from "@/lib/utils";
import {
	type ReactNode,
	createContext,
	useCallback,
	useContext,
	useEffect,
	useState,
	useSyncExternalStore,
} from "react";
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
	actorId: string;
	projectNameId: string;
	environmentNameId: string;
	endpoint?: string;
	enabled?: boolean;
	children: ReactNode;
}

export const ActorWorkerContextProvider = ({
	children,
	actorId,
	enabled,
	endpoint,
	projectNameId,
	environmentNameId,
}: ActorWorkerContextProviderProps) => {
	const [container] = useState<ActorWorkerContainer>(
		() => new ActorWorkerContainer(),
	);

	// biome-ignore lint/correctness/useExhaustiveDependencies: we want to create worker on each of those props change
	useEffect(() => {
		const ctrl = new AbortController();

		if (enabled && endpoint) {
			container.init({
				projectNameId,
				environmentNameId,
				actorId,
				endpoint,
				signal: ctrl.signal,
			});
		}

		return () => {
			ctrl.abort();
			container.terminate();
		};
	}, [actorId, projectNameId, environmentNameId, endpoint, enabled]);

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

export function useActorRpcs() {
	const container = useActorWorker();
	return useSyncExternalStore(
		useCallback(
			(cb) => {
				return container.subscribe(cb);
			},
			[container],
		),
		useCallback(() => {
			return container.getRpcs();
		}, [container]),
	);
}

export function useActorState() {
	const container = useActorWorker();
	return useSyncExternalStore(
		useCallback(
			(cb) => {
				return container.subscribe(cb);
			},
			[container],
		),
		useCallback(() => {
			return container.getState();
		}, [container]),
	);
}

export function useActorConnections() {
	const container = useActorWorker();
	return useSyncExternalStore(
		useCallback(
			(cb) => {
				return container.subscribe(cb);
			},
			[container],
		),
		useCallback(() => {
			return container.getConnections();
		}, [container]),
	);
}

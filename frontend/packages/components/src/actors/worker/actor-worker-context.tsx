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
import { assertNonNullable } from "../../lib/utils";
import { ActorFeature, type Actor, type ActorAtom } from "../actor-context";
import { selectAtom } from "jotai/utils";
import { useAtomValue } from "jotai";
import { toast } from "sonner";

export const ActorWorkerContext = createContext<ActorWorkerContainer | null>(
	null,
);

export const useActorWorker = () => {
	const value = useContext(ActorWorkerContext);
	assertNonNullable(value);
	return value;
};

const selector = (a: Actor) => ({
	actorId: a.id,
	endpoint: a.endpoint,
	enabled:
		!a.destroyedAt &&
		a.endpoint !== null &&
		a.startedAt !== null &&
		a.features?.includes(ActorFeature.Console),
});

interface ActorWorkerContextProviderProps {
	actor: ActorAtom;
	children: ReactNode;
	notifyOnReconnect?: boolean;
}

// FIXME: rewrite with jotai
export const ActorWorkerContextProvider = ({
	children,
	actor,
	notifyOnReconnect,
}: ActorWorkerContextProviderProps) => {
	const { actorId, endpoint, enabled } = useAtomValue(
		selectAtom(actor, selector),
	);

	const [container] = useState<ActorWorkerContainer>(
		() => new ActorWorkerContainer(),
	);

	// biome-ignore lint/correctness/useExhaustiveDependencies: we want to create worker on each of those props change
	useEffect(() => {
		const ctrl = new AbortController();

		if (enabled && endpoint) {
			container.init({
				actorId,
				endpoint,
				notifyOnReconnect,
				signal: ctrl.signal,
			});
		} else {
			toast.dismiss("ac-ws-reconnect");
		}

		return () => {
			ctrl.abort();
			container.terminate();
		};
	}, [actorId, endpoint, enabled]);

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

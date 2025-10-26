/** biome-ignore-all lint/correctness/useHookAtTopLevel: its safe */
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
import { useInspectorCredentials } from "@/app/credentials-context";
import { assertNonNullable } from "../../lib/utils";
import { useActor } from "../actor-queries-context";
import { useEngineCompatDataProvider } from "../data-provider";
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

const useInspectorToken = (runnerName: string) => {
	return match(__APP_TYPE__)
		.with("inspector", () => {
			return useInspectorCredentials().credentials?.token;
		})
		.otherwise(() => {
			const provider = useEngineCompatDataProvider();
			const { data } = useQuery(
				provider.runnerByNameQueryOptions({
					runnerName,
				}),
			);

			return (data?.metadata?.inspectorToken as string) || "";
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
	const dataProvider = useEngineCompatDataProvider();
	const engineToken = dataProvider.engineToken;
	const namespace = dataProvider.engineNamespace;
	const {
		data: {
			features,
			name,
			endpoint,
			destroyedAt,
			startedAt,
			sleepingAt,
			runner,
		} = {},
	} = useQuery(dataProvider.actorWorkerQueryOptions(actorId));
	const inspectorToken = useInspectorToken(runner || "");

	const enabled =
		(features?.includes(ActorFeature.Console) &&
			!destroyedAt &&
			!sleepingAt &&
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
				rpcs: rpcs ?? [],
				engineToken,
				runnerName: runner,
				namespace,
				inspectorToken,
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

/** biome-ignore-all lint/correctness/useHookAtTopLevel: safe guarded by build consts */
import { faPowerOff, faSpinnerThird, Icon } from "@rivet-gg/icons";
import {
	useInfiniteQuery,
	useMutation,
	useQuery,
	useSuspenseQuery,
} from "@tanstack/react-query";
import { useMatch, useRouteContext, useSearch } from "@tanstack/react-router";
import { createContext, type ReactNode, useContext, useMemo } from "react";
import { useLocalStorage } from "usehooks-ts";
import { DiscreteCopyButton } from "../copy-area";
import { getConfig } from "../lib/config";
import { ls } from "../lib/utils";
import { ShimmerLine } from "../shimmer-line";
import { Button } from "../ui/button";
import { useFiltersValue } from "./actor-filters-context";
import { Info } from "./actor-state-tab";
import { useDataProvider, useEngineCompatDataProvider } from "./data-provider";
import {
	ActorInspectorProvider as InspectorProvider,
	useActorInspector,
} from "./inspector-context";
import type { ActorId } from "./queries";

const InspectorGuardContext = createContext<ReactNode | null>(null);

export const useInspectorGuard = () => useContext(InspectorGuardContext);

interface GuardConnectableInspectorProps {
	actorId: ActorId;
	children: ReactNode;
}

export function GuardConnectableInspector({
	actorId,
	children,
}: GuardConnectableInspectorProps) {
	const filters = useFiltersValue({ onlyEphemeral: true });
	const {
		data: { destroyedAt, pendingAllocationAt, startedAt, sleepingAt } = {},
	} = useQuery({
		...useDataProvider().actorQueryOptions(actorId),
		refetchInterval: 1000,
		select: (data) => ({
			destroyedAt: data.destroyTs,
			sleepingAt: data.sleepTs,
			pendingAllocationAt: data.pendingAllocationTs,
			startedAt: data.startTs,
		}),
	});

	if (destroyedAt) {
		return (
			<InspectorGuardContext.Provider
				value={<Info>Unavailable for inactive Actors.</Info>}
			>
				{children}
			</InspectorGuardContext.Provider>
		);
	}

	if (pendingAllocationAt && !startedAt) {
		return (
			<InspectorGuardContext.Provider value={<NoRunners />}>
				{children}
			</InspectorGuardContext.Provider>
		);
	}

	if (sleepingAt) {
		if (filters.wakeOnSelect?.value?.[0] === "1") {
			return (
				<InspectorGuardContext.Provider
					value={
						<Info>
							<AutoWakeUpActor actorId={actorId} />
						</Info>
					}
				>
					{children}
				</InspectorGuardContext.Provider>
			);
		}
		return (
			<InspectorGuardContext.Provider
				value={
					<Info>
						<p>Unavailable for sleeping Actors.</p>
						<WakeUpActorButton actorId={actorId} />
					</Info>
				}
			>
				{children}
			</InspectorGuardContext.Provider>
		);
	}

	return (
		<ActorContextProvider actorId={actorId}>
			{children}
		</ActorContextProvider>
	);
}

function NoRunners() {
	const { data } = useInfiniteQuery({
		...useEngineCompatDataProvider().runnersQueryOptions(),
		refetchInterval: 5_000,
	});

	if (data?.length === 0) {
		return (
			<Info>
				<p>There are no runners.</p>
				<p>
					Check that you have at least one runner available to run
					your Actors.
				</p>
			</Info>
		);
	}
	return (
		<Info>
			<p>Cannot start Actor, runners are out of capacity.</p>
			<p>
				Add more runners to run the Actor or increase runner capacity.
			</p>
		</Info>
	);
}

function ActorContextProvider(props: {
	actorId: ActorId;
	children: ReactNode;
}) {
	const { data, isError } = useQuery(
		useDataProvider().actorInspectorTokenQueryOptions(props.actorId),
	);

	if (isError || !data) {
		return (
			<InspectorGuardContext.Provider
				value={
					<Info>
						<p>Unable to retrieve the Actor's Inspector token.</p>
						<p>
							Please verify that the Inspector is enabled for your
							Actor and that you are using the latest version of
							RivetKit.
						</p>
					</Info>
				}
			>
				{props.children}
			</InspectorGuardContext.Provider>
		);
	}

	return __APP_TYPE__ === "inspector" ? (
		<ActorInspectorProvider {...props} inspectorToken={data} />
	) : (
		<ActorEngineProvider {...props} inspectorToken={data} />
	);
}

function ActorInspectorProvider({
	actorId,
	inspectorToken,
	children,
}: {
	actorId: ActorId;
	inspectorToken: string;
	children: ReactNode;
}) {
	const url = useSearch({
		from: "/_context",
		select: (s) => s.u || "https://localhost:6420",
	});

	return (
		<InspectorProvider
			credentials={{ url: url, inspectorToken, token: "" }}
			actorId={actorId}
		>
			<InspectorGuard>{children}</InspectorGuard>
		</InspectorProvider>
	);
}

function useRunner(runnerName: string | undefined) {
	// check if its running
	const {
		data: hasRunner,
		isLoading,
		isSuccess,
	} = useQuery({
		...useEngineCompatDataProvider().runnerByNameQueryOptions({
			runnerName,
		}),
		enabled: !!runnerName,
		select: (data) => !!data,
		retryDelay: 10_000,
		refetchInterval: 1000,
	});

	// if not, check if its serverless
	const { data: hasServerlessRunner, isLoading: isLoadingServerlessRunners } =
		useQuery({
			...useEngineCompatDataProvider().runnerConfigQueryOptions({
				name: runnerName,
			}),
			enabled: !isSuccess && !!runnerName,
			retryDelay: 10_000,
			refetchInterval: 1000,
			select: (data) =>
				Object.values(data.datacenters).some((dc) => dc.serverless),
		});

	return {
		hasRunner: hasRunner || hasServerlessRunner,
		isLoading: isLoading || isLoadingServerlessRunners,
	};
}

function useActorRunner({ actorId }: { actorId: ActorId }) {
	const { data: actor, isLoading } = useSuspenseQuery(
		useDataProvider().actorQueryOptions(actorId),
	);

	const match = useMatch({
		from:
			__APP_TYPE__ === "engine"
				? "/_context/_engine/ns/$namespace"
				: "/_context/_cloud/orgs/$organization/projects/$project/ns/$namespace/",
		shouldThrow: false,
	});

	if (!match?.params.namespace || !actor.runnerNameSelector) {
		throw new Error("Actor is missing required fields");
	}

	const {
		data: runner,
		isLoading: isLoadingRunner,
		isSuccess,
	} = useQuery({
		...useEngineCompatDataProvider().runnerByNameQueryOptions({
			runnerName: actor.runnerNameSelector,
		}),
		retryDelay: 10_000,
		refetchInterval: 1000,
	});

	return {
		actor,
		runner,
		isLoading: isLoading || isLoadingRunner,
		isSuccess,
	};
}

function useEngineToken() {
	if (__APP_TYPE__ === "cloud") {
		const { data } = useQuery(
			useRouteContext({
				from: "/_context/_cloud/orgs/$organization/projects/$project/ns/$namespace",
			}).dataProvider.publishableTokenQueryOptions(),
		);
		return data;
	}
	const [data] = useLocalStorage(
		ls.engineCredentials.key(getConfig().apiUrl),
		"",
		{ serializer: JSON.stringify, deserializer: JSON.parse },
	);
	return data;
}

function useActorEngineContext({ actorId }: { actorId: ActorId }) {
	const { actor, runner, isLoading } = useActorRunner({ actorId });
	const engineToken = useEngineToken();

	const credentials = useMemo(() => {
		return {
			url: getConfig().apiUrl,
			token: engineToken,
		};
	}, [engineToken]);

	return { actor, runner, isLoading, credentials };
}

function ActorEngineProvider({
	actorId,
	inspectorToken,
	children,
}: {
	actorId: ActorId;
	children: ReactNode;
	inspectorToken: string;
}) {
	const { credentials, actor } = useActorEngineContext({
		actorId,
	});

	if (!actor.runnerNameSelector) {
		return (
			<InspectorGuardContext.Provider
				value={<NoRunnerInfo runner={"unknown"} />}
			>
				{children}
			</InspectorGuardContext.Provider>
		);
	}

	return (
		<InspectorProvider
			credentials={{ ...credentials, inspectorToken }}
			actorId={actorId}
		>
			<InspectorGuard>{children}</InspectorGuard>
		</InspectorProvider>
	);
}

function NoRunnerInfo({ runner }: { runner: string }) {
	return (
		<Info>
			<p>There are no runners connected to run this Actor.</p>
			<p>
				Check that your application is running and the
				runner&nbsp;name&nbsp;is&nbsp;
				<DiscreteCopyButton
					value={runner || ""}
					className="inline-block p-0 h-auto px-0.5 -mx-0.5"
				>
					<span className="font-mono-console">{runner}</span>
				</DiscreteCopyButton>
			</p>
		</Info>
	);
}

function WakeUpActorButton({ actorId }: { actorId: ActorId }) {
	const actorInspector = useActorInspector();
	const { runner } = useActorRunner({ actorId });

	const { mutate, isPending } = useMutation(
		actorInspector.actorWakeUpMutationOptions(actorId),
	);
	if (!runner) return null;
	return (
		<Button
			variant="outline"
			size="sm"
			onClick={() => mutate()}
			isLoading={isPending}
			startIcon={<Icon icon={faPowerOff} />}
		>
			Wake up Actor
		</Button>
	);
}

function AutoWakeUpActor({ actorId }: { actorId: ActorId }) {
	const actorInspector = useActorInspector();

	const { actor, runner } = useActorRunner({ actorId });
	const { hasRunner } = useRunner(actor.runnerNameSelector);

	useQuery(
		actorInspector.actorAutoWakeUpQueryOptions(actorId, {
			enabled: hasRunner,
		}),
	);

	if (!hasRunner)
		return <NoRunnerInfo runner={actor.runnerNameSelector || "unknown"} />;

	if (runner?.drainTs)
		return <NoRunnerInfo runner={actor.runnerNameSelector || "unknown"} />;

	return (
		<Info>
			<div className="flex items-center">
				<Icon icon={faSpinnerThird} className="animate-spin mr-2" />
				Waiting for Actor to wake...
			</div>
		</Info>
	);
}

function InspectorGuard({ children }: { children: ReactNode }) {
	const { connectionStatus } = useActorInspector();
	if (connectionStatus === "error") {
		return (
			<InspectorGuardContext.Provider
				value={
					<Info>
						<p>Unable to connect to the Actor's Inspector.</p>
						<p>
							Check that your Actor has the Inspector enabled and
							that your network allows connections to the
							Inspector URL.
						</p>
					</Info>
				}
			>
				{children}
			</InspectorGuardContext.Provider>
		);
	}

	return (
		<>
			{connectionStatus !== "connected" && <ShimmerLine />}
			{children}
		</>
	);
}

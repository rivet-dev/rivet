/** biome-ignore-all lint/correctness/useHookAtTopLevel: safe guarded by build consts */
import { faPowerOff, faSpinnerThird, Icon } from "@rivet-gg/icons";
import {
	useInfiniteQuery,
	useMutation,
	useQuery,
	useSuspenseQuery,
} from "@tanstack/react-query";
import { useMatch, useRouteContext } from "@tanstack/react-router";
import { createContext, type ReactNode, useContext, useMemo } from "react";
import { useLocalStorage } from "usehooks-ts";
import { useInspectorCredentials } from "@/app/credentials-context";
import { createInspectorActorContext } from "@/queries/actor-inspector";
import { queryClient } from "@/queries/global";
import { DiscreteCopyButton } from "../copy-area";
import { getConfig } from "../lib/config";
import { ls } from "../lib/utils";
import { Button } from "../ui/button";
import { useFiltersValue } from "./actor-filters-context";
import { ActorProvider, useActor } from "./actor-queries-context";
import { Info } from "./actor-state-tab";
import { useDataProvider, useEngineCompatDataProvider } from "./data-provider";
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
	const { data: { destroyedAt, pendingAllocationAt, startedAt } = {} } =
		useQuery({
			...useDataProvider().actorQueryOptions(actorId),
			refetchInterval: 1000,
			select: (data) => ({
				destroyedAt: data.destroyedAt,
				sleepingAt: data.sleepingAt,
				pendingAllocationAt: data.pendingAllocationAt,
				startedAt: data.startedAt,
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
	return __APP_TYPE__ === "inspector" ? (
		<ActorInspectorProvider {...props} />
	) : (
		<ActorEngineProvider {...props} />
	);
}

function ActorInspectorProvider({
	actorId,
	children,
}: {
	actorId: ActorId;
	children: ReactNode;
}) {
	const { credentials } = useInspectorCredentials();

	if (!credentials?.url || !credentials?.token) {
		throw new Error("Missing inspector credentials");
	}

	const actorContext = useMemo(() => {
		return createInspectorActorContext({
			...credentials,
		});
	}, [credentials]);

	return (
		<ActorProvider value={actorContext}>
			<InspectorGuard actorId={actorId}>{children}</InspectorGuard>
		</ActorProvider>
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

	if (!match?.params.namespace || !actor.runner) {
		throw new Error("Actor is missing required fields");
	}

	const {
		data: runner,
		isLoading: isLoadingRunner,
		isSuccess,
	} = useQuery({
		...useEngineCompatDataProvider().runnerByNameQueryOptions({
			runnerName: actor.runner,
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
	const provider = useEngineCompatDataProvider();

	const actorContext = useMemo(() => {
		return createInspectorActorContext({
					url: getConfig().apiUrl,
					token: async () => {
						const runner = await queryClient.fetchQuery(
							provider.runnerByNameQueryOptions({
								runnerName: actor?.runner || "",
							}),
						);
						return (
							(runner?.metadata?.inspectorToken as string) || ""
						);
					},
					engineToken,
				});
	}, [actorId, actor?.runner, provider.runnerByNameQueryOptions, engineToken]);

	return { actorContext, actor, runner, isLoading };
}

function ActorEngineProvider({
	actorId,
	children,
}: {
	actorId: ActorId;
	children: ReactNode;
}) {
	const { actorContext, actor } = useActorEngineContext({
		actorId,
	});

	if (!actor.runner) {
		return (
			<InspectorGuardContext.Provider
				value={<NoRunnerInfo runner={"unknown"} />}
			>
				{children}
			</InspectorGuardContext.Provider>
		);
	}

	return (
		<ActorProvider value={actorContext}>
			<InspectorGuard actorId={actorId}>{children}</InspectorGuard>
		</ActorProvider>
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
	const actorContext = useActor();
	const { runner } = useActorRunner({ actorId });

	const { mutate, isPending } = useMutation(
		actorContext.actorWakeUpMutationOptions(actorId),
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
	const actorContext = useActor();

	const { actor, runner } = useActorRunner({ actorId });
	const { hasRunner } = useRunner(actor.runner);

	useQuery(
		actorContext.actorAutoWakeUpQueryOptions(actorId, {
			enabled: hasRunner,
		}),
	);

	if (!hasRunner) return <NoRunnerInfo runner={actor.runner || "unknown"} />;

	if (runner?.drainTs)
		return <NoRunnerInfo runner={actor.runner || "unknown"} />;

	return (
		<Info>
			<div className="flex items-center">
				<Icon icon={faSpinnerThird} className="animate-spin mr-2" />
				Waiting for Actor to wake...
			</div>
		</Info>
	);
}

function InspectorGuard({
	actorId,
	children,
}: {
	actorId: ActorId;
	children: ReactNode;
}) {
	const filters = useFiltersValue({ includeEphemeral: true });

	const { data: { sleepingAt } = {} } = useQuery({
		...useDataProvider().actorQueryOptions(actorId),
		refetchInterval: 1000,
		select: (data) => ({
			destroyedAt: data.destroyedAt,
			sleepingAt: data.sleepingAt,
			pendingAllocationAt: data.pendingAllocationAt,
			startedAt: data.startedAt,
		}),
	});

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
		<InspectorGuardInner actorId={actorId}>{children}</InspectorGuardInner>
	);
}

function InspectorGuardInner({
	actorId,
	children,
}: {
	actorId: ActorId;
	children: ReactNode;
}) {
	const { isError } = useQuery({
		...useActor().actorPingQueryOptions(actorId),
		retryDelay: 10_000,
		enabled: true,
	});
	if (isError) {
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

	return children;
}

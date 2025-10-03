import { faPowerOff, faSpinnerThird, Icon } from "@rivet-gg/icons";
import { useMutation, useQuery, useSuspenseQuery } from "@tanstack/react-query";
import { useMatch } from "@tanstack/react-router";
import {
	createContext,
	type ReactNode,
	useContext,
	useMemo,
	useState,
} from "react";
import { useInspectorCredentials } from "@/app/credentials-context";
import { createInspectorActorContext } from "@/queries/actor-inspector";
import { DiscreteCopyButton } from "../copy-area";
import { getConfig } from "../lib/config";
import { ls } from "../lib/utils";
import { Button } from "../ui/button";
import { useFiltersValue } from "./actor-filters-context";
import { ActorProvider } from "./actor-queries-context";
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
	const filters = useFiltersValue({ includeEphemeral: true });
	const {
		data: { destroyedAt, sleepingAt, pendingAllocationAt, startedAt } = {},
	} = useQuery({
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

	if (pendingAllocationAt && !startedAt) {
		return (
			<InspectorGuardContext.Provider
				value={
					<Info>
						<p>Cannot start Actor, runners are out of capacity.</p>
						<p>
							Add more runners to run the Actor or increase runner
							capacity.
						</p>
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

function ActorInspectorProvider({ children }: { children: ReactNode }) {
	const { credentials } = useInspectorCredentials();

	if (!credentials?.url || !credentials?.token) {
		throw new Error("Missing inspector credentials");
	}

	const actorContext = useMemo(() => {
		return createInspectorActorContext({
			...credentials,
		});
	}, [credentials]);

	return <ActorProvider value={actorContext}>{children}</ActorProvider>;
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
			namespace: match.params.namespace,
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

function useActorEngineContext({ actorId }: { actorId: ActorId }) {
	const { actor, runner, isLoading } = useActorRunner({ actorId });

	const actorContext = useMemo(() => {
		return createInspectorActorContext({
			url: getConfig().apiUrl,
			token: (runner?.metadata?.inspectorToken as string) || "",
			engineToken: ls.engineCredentials.get(getConfig().apiUrl) || "",
		});
	}, [runner?.metadata?.inspectorToken]);

	return { actorContext, actor, runner, isLoading };
}

function ActorEngineProvider({
	actorId,
	children,
}: {
	actorId: ActorId;
	children: ReactNode;
}) {
	const { actorContext, actor, runner } = useActorEngineContext({
		actorId,
	});

	if (!runner || !actor.runner) {
		return (
			<InspectorGuardContext.Provider
				value={
					<NoRunnerInfo
						runner={runner?.name || actor.runner || "unknown"}
					/>
				}
			>
				{children}
			</InspectorGuardContext.Provider>
		);
	}

	return <ActorProvider value={actorContext}>{children}</ActorProvider>;
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
			<p>Will retry automatically in the background.</p>
		</Info>
	);
}

function WakeUpActorButton({ actorId }: { actorId: ActorId }) {
	const { runner, actorContext } = useActorEngineContext({ actorId });

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
	const { runner, actor, actorContext } = useActorEngineContext({
		actorId,
	});

	useQuery(
		actorContext.actorAutoWakeUpQueryOptions(actorId, {
			enabled: !!runner,
		}),
	);

	if (!runner) return <NoRunnerInfo runner={actor.runner || "unknown"} />;

	return (
		<Info>
			<div className="flex items-center">
				<Icon icon={faSpinnerThird} className="animate-spin mr-2" />
				Waiting for Actor to wake...
			</div>
		</Info>
	);
}

import { faPowerOff, faSpinnerThird, Icon } from "@rivet-gg/icons";
import { useMutation, useQuery, useSuspenseQuery } from "@tanstack/react-query";
import { useMatch } from "@tanstack/react-router";
import { type ReactNode, useMemo } from "react";
import { useInspectorCredentials } from "@/app/credentials-context";
import { createEngineActorContext } from "@/queries/actor-engine";
import { createInspectorActorContext } from "@/queries/actor-inspector";
import {
	type NamespaceNameId,
	runnerByNameQueryOptions,
} from "@/queries/manager-engine";
import { DiscreteCopyButton } from "../copy-area";
import { Button } from "../ui/button";
import { useFiltersValue } from "./actor-filters-context";
import { ActorProvider } from "./actor-queries-context";
import { Info } from "./actor-state-tab";
import { useManager } from "./manager-context";
import type { ActorId } from "./queries";

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
		...useManager().actorQueryOptions(actorId),
		refetchInterval: 1000,
	});

	if (destroyedAt) {
		return <Info>Unavailable for inactive Actors.</Info>;
	}

	if (sleepingAt) {
		if (filters.wakeOnSelect?.value?.[0] === "1") {
			return (
				<Info>
					<AutoWakeUpActor actorId={actorId} />
				</Info>
			);
		}
		return (
			<Info>
				<p>Unavailable for sleeping Actors.</p>
				<WakeUpActorButton actorId={actorId} />
			</Info>
		);
	}

	if (pendingAllocationAt && !startedAt) {
		return (
			<Info>
				Cannot start Actor, runners are out of capacity. Add more
				runners to run the Actor or increase runner capacity.
			</Info>
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

function ActorInspectorProvider({
	actorId,
	children,
}: {
	actorId: ActorId;
	children: ReactNode;
}) {
	const { data } = useSuspenseQuery(useManager().actorQueryOptions(actorId));
	const { credentials } = useInspectorCredentials();

	if (!credentials?.url || !credentials?.token) {
		throw new Error("Missing inspector credentials");
	}

	const actorContext = useMemo(() => {
		return createInspectorActorContext({
			...credentials,
			name: data.name || "",
		});
	}, [credentials, data.name]);

	return <ActorProvider value={actorContext}>{children}</ActorProvider>;
}

function useActorRunner({ actorId }: { actorId: ActorId }) {
	const { data: actor } = useSuspenseQuery(
		useManager().actorQueryOptions(actorId),
	);

	const match = useMatch({
		from: "/_layout/ns/$namespace",
	});

	if (!match.params.namespace || !actor.runner) {
		throw new Error("Actor is missing required fields");
	}

	const { data: runner } = useQuery({
		...runnerByNameQueryOptions({
			runnerName: actor.runner,
			namespace: match.params.namespace as NamespaceNameId,
		}),
		refetchInterval: 1000,
	});

	return { actor, runner };
}

function useActorEngineContext({ actorId }: { actorId: ActorId }) {
	const { actor, runner } = useActorRunner({ actorId });

	const actorContext = useMemo(() => {
		return createEngineActorContext({
			token: (runner?.metadata?.inspectorToken as string) || "",
		});
	}, [runner?.metadata?.inspectorToken]);

	return { actorContext, actor, runner };
}

function ActorEngineProvider({
	actorId,
	children,
}: {
	actorId: ActorId;
	children: ReactNode;
}) {
	const { actorContext, actor, runner } = useActorEngineContext({ actorId });

	if (!runner || !actor.runner) {
		return (
			<NoRunnerInfo runner={runner?.name || actor.runner || "unknown"} />
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
	const { runner, actor, actorContext } = useActorEngineContext({ actorId });

	const { isPending } = useQuery(
		actorContext.actorAutoWakeUpQueryOptions(actorId, {
			enabled: !!runner,
		}),
	);

	if (!runner) return <NoRunnerInfo runner={actor.runner || "unknown"} />;

	return isPending ? (
		<Info>
			<div className="flex items-center">
				<Icon icon={faSpinnerThird} className="animate-spin mr-2" />
				Waiting for Actor to wake...
			</div>
		</Info>
	) : null;
}

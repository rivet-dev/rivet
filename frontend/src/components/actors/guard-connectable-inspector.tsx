/** biome-ignore-all lint/correctness/useHookAtTopLevel: safe guarded by build consts */

import {
	faExclamationTriangle,
	faPowerOff,
	faSpinnerThird,
	Icon,
} from "@rivet-gg/icons";
import {
	useInfiniteQuery,
	useMutation,
	useQuery,
	useSuspenseQuery,
} from "@tanstack/react-query";
import { useRouteContext, useSearch } from "@tanstack/react-router";
import { createContext, type ReactNode, useContext, useMemo } from "react";
import { match, P } from "ts-pattern";
import { useLocalStorage } from "usehooks-ts";
import { isRivetApiError } from "@/lib/errors";
import { DiscreteCopyButton } from "../copy-area";
import { getConfig, useConfig } from "../lib/config";
import { ls } from "../lib/utils";
import { ShimmerLine } from "../shimmer-line";
import { Button } from "../ui/button";
import { useFiltersValue } from "./actor-filters-context";
import {
	actorWakeUpMutationOptions,
	actorWakeUpQueryOptions,
	ActorInspectorProvider as InspectorProvider,
	useActorInspector,
} from "./actor-inspector-context";
import { Info } from "./actor-state-tab";
import { ErrorDetails, QueriedActorError } from "./actor-status-label";
import { useDataProvider, useEngineCompatDataProvider } from "./data-provider";
import { useActorInspectorData } from "./hooks/use-actor-inspector-data";
import type { ActorId, ActorStatus } from "./queries";

const InspectorGuardContext = createContext<ReactNode | null>(null);

const RIVET_KIT_MIN_VERSION = "2.0.35";

export const useInspectorGuard = () => useContext(InspectorGuardContext);

interface GuardConnectableInspectorProps {
	actorId: ActorId;
	children: ReactNode;
}

export function GuardConnectableInspector({
	actorId,
	children,
}: GuardConnectableInspectorProps) {
	const { data: status } = useQuery({
		...useDataProvider().actorStatusQueryOptions(actorId),
		refetchInterval: 1000,
	});

	return match(status)
		.with(P.union("running"), () => (
			<ActorContextProvider actorId={actorId}>
				{children}
			</ActorContextProvider>
		))
		.otherwise((status) => (
			<InspectorGuardContext.Provider
				value={<UnavailableInfo actorId={actorId} status={status} />}
			>
				{children}
			</InspectorGuardContext.Provider>
		));
}

function UnavailableInfo({
	actorId,
	status,
}: {
	actorId: ActorId;
	status?: ActorStatus;
}) {
	return match(status)
		.with("crashed", () => (
			<Info>
				<Icon
					icon={faExclamationTriangle}
					className="text-4xl text-destructive"
				/>
				<p>Actor is unavailable.</p>

				<QueriedActorError actorId={actorId} />
			</Info>
		))
		.with("pending", () => <NoRunners />)
		.with("stopped", () => (
			<Info>
				<p>Actor has been destroyed.</p>
			</Info>
		))
		.with("sleeping", () => <SleepingActor actorId={actorId} />)
		.with("starting", () => (
			<Info>
				<div className="flex items-center">
					<Icon icon={faSpinnerThird} className="animate-spin mr-2" />
					Actor is starting...
				</div>
			</Info>
		))
		.otherwise(() => {
			return (
				<Info>
					<p>Actor is unavailable.</p>
				</Info>
			);
		});
}

function SleepingActor({ actorId }: { actorId: ActorId }) {
	const { wakeOnSelect } = useFiltersValue({ onlyEphemeral: true });

	if (wakeOnSelect?.value[0] === "1") {
		return <AutoWakeUpActor actorId={actorId} />;
	}

	return <WakeUpActorButton actorId={actorId} />;
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
	const { isError, token, isLoading, metadata, error } =
		useActorInspectorData(props.actorId);

	if (isLoading) {
		return (
			<InspectorGuardContext.Provider value={<ConnectingInspector />}>
				{props.children}
			</InspectorGuardContext.Provider>
		);
	}

	if (!token || !metadata || isError) {
		return (
			<InspectorGuardContext.Provider
				value={
					<Info>
						<p>Unable to retrieve the Actor's Inspector token.</p>
						<p>
							Please ensure the Inspector is enabled for your
							Actor and that you're using RivetKit version{" "}
							<b className="font-mono-console">
								{RIVET_KIT_MIN_VERSION}
							</b>{" "}
							or newer.
						</p>
						<p>
							Current RivetKit version:{" "}
							<DiscreteCopyButton
								value={metadata?.version || "unknown"}
								className="inline-block p-0 h-auto px-0.5"
							>
								<span className="font-mono-console text-lg font-bold">
									{metadata?.version || "unknown"}
								</span>
							</DiscreteCopyButton>
						</p>
						<ErrorDetails
							error={isRivetApiError(error) ? error.body : error}
						/>
					</Info>
				}
			>
				{props.children}
			</InspectorGuardContext.Provider>
		);
	}

	return __APP_TYPE__ === "inspector" ? (
		<ActorInspectorProvider {...props} inspectorToken={token} />
	) : (
		<ActorEngineProvider {...props} inspectorToken={token} />
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
			<InspectorGuard actorId={actorId}>{children}</InspectorGuard>
		</InspectorProvider>
	);
}

function useActorRunner({ actorId }: { actorId: ActorId }) {
	const { data: actor, isLoading } = useSuspenseQuery(
		useDataProvider().actorQueryOptions(actorId),
	);

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
	if (__APP_TYPE__ === "inspector") {
		return "";
	}
	const [data] = useLocalStorage(
		ls.engineCredentials.key(getConfig().apiUrl),
		"",
		{ serializer: JSON.stringify, deserializer: JSON.parse },
	);
	return data?.token || "";
}

function useEngineUrl() {
	if (__APP_TYPE__ === "inspector") {
		return (
			useSearch({
				from: "/_context",
				select: (s) => s.u,
			}) || "https://localhost:6420"
		);
	}

	return useConfig().apiUrl;
}

function useActorEngineContext({ actorId }: { actorId: ActorId }) {
	const { actor, runner, isLoading } = useActorRunner({ actorId });
	const engineToken = useEngineToken();
	const url = useEngineUrl();

	const credentials = useMemo(() => {
		return {
			url,
			token: engineToken,
		};
	}, [url, engineToken]);

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
			<InspectorGuard actorId={actorId}>{children}</InspectorGuard>
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
	const { credentials } = useActorEngineContext({
		actorId,
	});

	const { mutate, isPending } = useMutation(actorWakeUpMutationOptions());

	return (
		<Info>
			<p>Actor is sleeping.</p>
			<Button
				variant="outline"
				size="sm"
				onClick={() =>
					mutate({
						actorId,
						credentials,
					})
				}
				isLoading={isPending}
				startIcon={<Icon icon={faPowerOff} />}
			>
				Wake up Actor
			</Button>
		</Info>
	);
}

function AutoWakeUpActor({ actorId }: { actorId: ActorId }) {
	const { credentials } = useActorEngineContext({
		actorId,
	});

	useQuery({
		...actorWakeUpQueryOptions({ actorId, credentials }),
		retryDelay: 10_000,
		refetchInterval: 1_000,
	});

	return (
		<Info>
			<div className="flex items-center">
				<Icon icon={faSpinnerThird} className="animate-spin mr-2" />
				Waiting for Actor to wake...
			</div>
		</Info>
	);
}

function ConnectingInspector() {
	return (
		<Info>
			<div className="flex items-center">
				<Icon icon={faSpinnerThird} className="animate-spin mr-2" />
				Connecting to Inspector...
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
	const {
		connectionStatus,
		isInspectorAvailable,
		actorMetadataQueryOptions,
	} = useActorInspector();

	const { data, isLoading, error } = useQuery(
		actorMetadataQueryOptions(actorId),
	);

	if (!data || error || !isInspectorAvailable) {
		return <OutdatedInspector>{children}</OutdatedInspector>;
	}

	if (isLoading) {
		return (
			<InspectorGuardContext.Provider value={<ConnectingInspector />}>
				{children}
			</InspectorGuardContext.Provider>
		);
	}

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

function OutdatedInspector({ children }: { children: ReactNode }) {
	return (
		<InspectorGuardContext.Provider
			value={
				<Info>
					<p>
						Please upgrade your Actor to RivetKit version{" "}
						<b className="font-mono-console">
							{RIVET_KIT_MIN_VERSION}
						</b>{" "}
						to use the Inspector.
					</p>
				</Info>
			}
		>
			{children}
		</InspectorGuardContext.Provider>
	);
}

/** biome-ignore-all lint/correctness/useHookAtTopLevel: safe guarded by build consts */

import {
	faExclamationTriangle,
	faPowerOff,
	faQuestionCircle,
	faSpinnerThird,
	Icon,
} from "@rivet-gg/icons";
import * as Sentry from "@sentry/react";
import {
	useInfiniteQuery,
	useMutation,
	useQuery,
	useSuspenseQuery,
} from "@tanstack/react-query";
import { useRouteContext, useSearch } from "@tanstack/react-router";
import {
	createContext,
	type ReactNode,
	useContext,
	useEffect,
	useMemo,
} from "react";
import { match, P } from "ts-pattern";
import { useLocalStorage } from "usehooks-ts";
import { HelpDropdown } from "@/app/help-dropdown";
import { isRivetApiError } from "@/lib/errors";
import { features } from "@/lib/features";
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
import {
	ErrorDetails,
	QueriedActorError,
	QueriedActorStatusAdditionalInfo,
} from "./actor-status-label";
import { useDataProvider, useEngineCompatDataProvider } from "./data-provider";
import { useActorInspectorData } from "./hooks/use-actor-inspector-data";
import type { ActorId, ActorStatus } from "./queries";

const InspectorGuardContext = createContext<ReactNode | null>(null);

const RIVET_KIT_MIN_VERSION = "2.0.35";

/**
 * Compare semantic versions including pre-release versions.
 * Returns true if version1 < version2.
 * Handles versions like "2.0.35", "2.1.0-rc.2", "2.1.0-alpha.1"
 */
function isVersionOutdated(version1: string, version2: string): boolean {
	// Preview builds (e.g. "0.0.0-pr.4673.6d") are never considered outdated.
	if (version1.startsWith("0.0.0-") || version2.startsWith("0.0.0-")) {
		return false;
	}

	// Extract base version and pre-release info
	const parseVersion = (v: string) => {
		const [baseStr, prerelease] = v.split("-");
		const parts = baseStr.split(".").map(Number);
		return {
			major: parts[0] || 0,
			minor: parts[1] || 0,
			patch: parts[2] || 0,
			prerelease,
		};
	};

	const v1 = parseVersion(version1);
	const v2 = parseVersion(version2);

	// Compare major.minor.patch
	if (v1.major !== v2.major) return v1.major < v2.major;
	if (v1.minor !== v2.minor) return v1.minor < v2.minor;
	if (v1.patch !== v2.patch) return v1.patch < v2.patch;

	// Base versions are equal - compare pre-release
	// Pre-release versions (e.g., rc, alpha) are older than release versions
	const hasPrerelease1 = !!v1.prerelease;
	const hasPrerelease2 = !!v2.prerelease;

	if (hasPrerelease1 && !hasPrerelease2) return true; // "2.0.0-rc.1" < "2.0.0"
	if (!hasPrerelease1 && hasPrerelease2) return false; // "2.0.0" > "2.0.0-rc.1"

	return false; // versions are equal
}

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

				<div className="flex gap-4 items-center mt-4">
					<WakeUpActorButton actorId={actorId} />

					<HelpDropdown>
						<Button
							size="sm"
							variant="outline"
							startIcon={<Icon icon={faQuestionCircle} />}
						>
							Need help?
						</Button>
					</HelpDropdown>
				</div>
			</Info>
		))
		.with("crash-loop", () => <CrashLoopActor actorId={actorId} />)
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

	return (
		<Info>
			<p>Actor is sleeping.</p>
			<WakeUpActorButton actorId={actorId} />
		</Info>
	);
}

function CrashLoopActor({ actorId }: { actorId: ActorId }) {
	const { data: { rescheduleTs } = {} } = useQuery({
		...useDataProvider().actorStatusAdditionalInfoQueryOptions(actorId),
		refetchInterval: 5_000,
	});

	const rescheduleInFuture = rescheduleTs && rescheduleTs > Date.now();

	return (
		<Info>
			<Icon
				icon={faExclamationTriangle}
				className="text-4xl text-destructive"
			/>
			<p>Actor is failing to start and will be retried.</p>
			<QueriedActorStatusAdditionalInfo actorId={actorId} />

			<div className="flex gap-4 items-center mt-4">
				{!rescheduleInFuture && <WakeUpActorButton actorId={actorId} />}
				<HelpDropdown>
					<Button
						size="sm"
						variant="outline"
						startIcon={<Icon icon={faQuestionCircle} />}
					>
						Need help?
					</Button>
				</HelpDropdown>
			</div>
		</Info>
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

interface InspectorTokenErrorContext {
	statusCode?: number;
	metadata?: { version?: string; type?: string };
	error?: unknown;
}

function buildInspectorTokenErrorMessage(
	context: InspectorTokenErrorContext,
): ReactNode {
	const { statusCode, metadata, error } = context;

	const isLocal = metadata?.type === "local";

	// 403: Token not set in run config (deployed only)
	if (statusCode === 403 && !isLocal) {
		return (
			<Info>
				<p>
					Inspector token not configured. To enable the Inspector in
					your deployed environment, set the token in your run config:
				</p>
				<code className="block bg-gray-100 p-3 rounded mt-2 text-sm">
					RIVET_INSPECTOR_TOKEN=&lt;your-token&gt;
				</code>
				<p className="mt-2 text-sm text-gray-600">
					See{" "}
					<a
						href="https://rivet.dev/docs/actors/inspector"
						className="text-blue-600 hover:underline"
					>
						Inspector documentation
					</a>{" "}
					for more details.
				</p>
			</Info>
		);
	}

	// 404: Check version mismatch first
	if (statusCode === 404) {
		if (metadata?.version) {
			const currentVersion = metadata.version;
			const minVersion = RIVET_KIT_MIN_VERSION;
			const versionIsOutdated =
				currentVersion &&
				minVersion &&
				isVersionOutdated(currentVersion, minVersion);

			if (versionIsOutdated) {
				return (
					<Info>
						<p>RivetKit version is outdated.</p>
						<p>
							Your RivetKit version (
							<span className="font-mono-console font-bold">
								{currentVersion}
							</span>
							) is older than the required version (
							<span className="font-mono-console font-bold">
								{minVersion}
							</span>
							).
						</p>
						<p className="mt-2">
							Please upgrade RivetKit to use the Inspector.
						</p>
					</Info>
				);
			}
		}

		// 404 in deployed environment but not a version issue
		if (!isLocal) {
			return (
				<Info>
					<p>
						Inspector token endpoint returned a 404. Please check
						your RivetKit version and contact support if the issue
						persists.
					</p>
					<p className="mt-2 text-sm text-gray-600">
						Current RivetKit version:{" "}
						<span className="font-mono-console">
							{metadata?.version || "unknown"}
						</span>
					</p>
				</Info>
			);
		}

		// 404 in local environment
		return (
			<Info>
				<p>
					Inspector token endpoint returned a 404. This might indicate
					an outdated version of RivetKit.
				</p>
				<p className="mt-2 text-sm text-gray-600">
					Current RivetKit version:{" "}
					<span className="font-mono-console">
						{metadata?.version || "unknown"}
					</span>
				</p>
				<p className="mt-2">
					Please ensure you're running the latest version of RivetKit.
				</p>
				<p className="mt-2 text-sm">
					If the problem persists, please contact support.
				</p>
			</Info>
		);
	}

	// Unknown error code in deployed environment
	if (statusCode && statusCode !== 403 && statusCode !== 404 && !isLocal) {
		return <UnexpectedInspectorError error={error} metadata={metadata} />;
	}

	// Default/generic error
	return (
		<Info>
			<p>Unable to retrieve the Actor's Inspector token.</p>
			<p>
				Please ensure the Inspector is enabled for your Actor and that
				you're using RivetKit version{" "}
				<b className="font-mono-console">{RIVET_KIT_MIN_VERSION}</b> or
				newer.
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
			<ErrorDetails error={isRivetApiError(error) ? error.body : error} />
		</Info>
	);
}

function UnexpectedInspectorError({
	error,
	metadata,
}: {
	error: unknown;
	metadata?: { version?: string; type?: string };
}) {
	// biome-ignore lint/correctness/useExhaustiveDependencies: we only want to log on initial error, not on metadata changes
	useEffect(() => {
		Sentry.captureException(error, {
			contexts: {
				inspector: {
					error,
					type: metadata?.type,
					version: metadata?.version,
				},
			},
			tags: {
				component: "ActorContextProvider",
			},
		});
	}, []);
	return (
		<Info>
			<p>
				An unexpected error occurred while connecting to the Inspector.
			</p>
			<p className="mt-2">
				Our team has been notified. Please try again later or contact
				support if the issue persists.
			</p>
			<ErrorDetails error={error} />
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
		const statusCode = isRivetApiError(error)
			? (error.statusCode as number)
			: undefined;

		const errorMessage = buildInspectorTokenErrorMessage({
			statusCode,
			metadata,
			error,
		});

		return (
			<InspectorGuardContext.Provider value={errorMessage}>
				{props.children}
			</InspectorGuardContext.Provider>
		);
	}

	return <ActorEngineProvider {...props} inspectorToken={token} />;
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
	if (features.multitenancy) {
		const { data } = useQuery(
			useRouteContext({
				from: "/_context/orgs/$organization/projects/$project/ns/$namespace",
			}).dataProvider.publishableTokenQueryOptions(),
		);
		return data;
	}
	const [data] = useLocalStorage(
		ls.engineCredentials.key(getConfig().apiUrl),
		"",
		{ serializer: JSON.stringify, deserializer: JSON.parse },
	);
	return data?.token || "";
}

function useEngineUrl() {
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

	const { data, isPending, error } = useQuery(
		actorMetadataQueryOptions(actorId),
	);

	if (isPending) {
		return (
			<InspectorGuardContext.Provider value={<ConnectingInspector />}>
				{children}
			</InspectorGuardContext.Provider>
		);
	}

	if (!data || error || !isInspectorAvailable) {
		return <OutdatedInspector>{children}</OutdatedInspector>;
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

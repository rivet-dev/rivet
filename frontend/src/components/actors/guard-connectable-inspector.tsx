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
import { z } from "zod";
import { HelpDropdown } from "@/app/help-dropdown";
import { isRivetApiError } from "@/lib/errors";
import { features } from "@/lib/features";
import { DiscreteCopyButton } from "../copy-area";
import { useConfig } from "../lib/config";
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

const EngineErrorBodySchema = z.object({
	group: z.string(),
	code: z.string(),
	message: z.string(),
	metadata: z.unknown().optional(),
});

type EngineErrorBody = z.infer<typeof EngineErrorBodySchema>;

function extractEngineErrorBody(error: unknown): EngineErrorBody | undefined {
	if (!isRivetApiError(error)) return undefined;
	const parsed = EngineErrorBodySchema.safeParse(error.body);
	return parsed.success ? parsed.data : undefined;
}

export function buildInspectorTokenErrorMessage(
	context: InspectorTokenErrorContext,
): ReactNode {
	const { statusCode, metadata, error } = context;

	const engineError = extractEngineErrorBody(error);

	// Version check is orthogonal to the specific error. If RivetKit itself
	// is below the minimum, surface that first regardless of which call
	// failed.
	if (
		metadata?.version &&
		isVersionOutdated(metadata.version, RIVET_KIT_MIN_VERSION)
	) {
		return (
			<OutdatedRivetKit
				currentVersion={metadata.version}
				error={error}
			/>
		);
	}

	// The inspector token couldn't be retrieved. Two underlying causes:
	// 1. Engine route is missing entirely (older engine without the KV
	//    endpoint, returns 404 with no structured body).
	// 2. Engine returned `actor.kv_key_not_found` for the inspector token
	//    key.
	const isKvKeyMissing =
		engineError?.group === "actor" &&
		engineError?.code === "kv_key_not_found";
	const isEndpointMissing = statusCode === 404 && !engineError;

	if (isKvKeyMissing || isEndpointMissing) {
		return (
			<MissingInspectorToken
				currentVersion={metadata?.version}
				error={error}
			/>
		);
	}

	// Inspector auth rejected. Deployed only: locally we expect to fall
	// through to the verbose error so the user can debug.
	const isLocal = metadata?.type === "local";
	if (statusCode === 403 && !isLocal) {
		return (
			<Info>
				<Icon
					icon={faExclamationTriangle}
					className="text-4xl text-destructive"
				/>
				<p className="font-semibold">Inspector authentication failed.</p>
				<p className="text-sm text-muted-foreground">
					The dashboard fetches the per-actor inspector token from the
					engine KV API. This typically indicates a permissions or
					configuration issue with the request. See the{" "}
					<a
						href="https://rivet.dev/docs/actors/inspector"
						className="underline hover:text-foreground"
					>
						Inspector documentation
					</a>{" "}
					for more details.
				</p>
				{engineError ? (
					<EngineErrorBlock body={engineError} />
				) : (
					<ErrorDetails
						error={isRivetApiError(error) ? error.body : error}
					/>
				)}
			</Info>
		);
	}

	// Surface the engine's structured error directly.
	if (engineError) {
		return (
			<Info>
				<Icon
					icon={faExclamationTriangle}
					className="text-4xl text-destructive"
				/>
				<p className="font-semibold">
					Unable to connect to the Inspector.
				</p>
				<EngineErrorBlock body={engineError} />
			</Info>
		);
	}

	return <UnexpectedInspectorError error={error} metadata={metadata} />;
}

function OutdatedRivetKit({
	currentVersion,
	error,
}: {
	currentVersion: string;
	error?: unknown;
}) {
	return (
		<Info>
			<Icon
				icon={faExclamationTriangle}
				className="text-4xl text-destructive"
			/>
			<p className="font-semibold">RivetKit version is outdated.</p>
			<p className="text-sm text-muted-foreground">
				Your RivetKit version (
				<span className="font-mono-console">{currentVersion}</span>) is
				older than the required version (
				<span className="font-mono-console">
					{RIVET_KIT_MIN_VERSION}
				</span>
				). Please upgrade RivetKit to use the Inspector.
			</p>
			{error ? (
				<ErrorDetails
					error={isRivetApiError(error) ? error.body : error}
				/>
			) : null}
		</Info>
	);
}

function MissingInspectorToken({
	currentVersion,
	error,
}: {
	currentVersion?: string;
	error: unknown;
}) {
	return (
		<Info>
			<Icon
				icon={faExclamationTriangle}
				className="text-4xl text-destructive"
			/>
			<p className="font-semibold">Inspector token not found.</p>
			<p className="text-sm text-muted-foreground">
				Ensure the Inspector is enabled on your Actor and that you are
				running RivetKit{" "}
				<span className="font-mono-console">
					{RIVET_KIT_MIN_VERSION}
				</span>{" "}
				or newer.
			</p>
			<p className="text-sm text-muted-foreground">
				Current RivetKit version:{" "}
				<DiscreteCopyButton
					value={currentVersion || "unknown"}
					className="inline-block p-0 h-auto px-0.5"
				>
					<span className="font-mono-console">
						{currentVersion || "unknown"}
					</span>
				</DiscreteCopyButton>
			</p>
			<ErrorDetails error={isRivetApiError(error) ? error.body : error} />
		</Info>
	);
}

function EngineErrorBlock({ body }: { body: EngineErrorBody }) {
	return (
		<div className="mt-2 flex flex-col items-center gap-2 w-full">
			<span className="font-mono-console text-xs px-2 py-0.5 rounded bg-muted text-muted-foreground">
				{body.group}.{body.code}
			</span>
			<p className="text-sm text-muted-foreground">{body.message}</p>
			<ErrorDetails error={body} />
		</div>
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
			<Icon
				icon={faExclamationTriangle}
				className="text-4xl text-destructive"
			/>
			<p className="font-semibold">
				Unable to connect to the Inspector.
			</p>
			<p className="text-sm text-muted-foreground">
				An unexpected error occurred. Our team has been notified.
				Please try again later or contact support if the issue
				persists.
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
	if (features.platform) {
		const { data } = useQuery(
			useRouteContext({
				from: "/_context/orgs/$organization/projects/$project/ns/$namespace",
			}).dataProvider.publishableTokenQueryOptions(),
		);
		return data || "";
	}
	const { data } = useQuery(
		useEngineCompatDataProvider().engineAdminTokenQueryOptions(),
	);
	return data || "";
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
		return (
			<OutdatedInspector error={error}>{children}</OutdatedInspector>
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

export function OutdatedInspector({
	children,
	error,
}: {
	children: ReactNode;
	error?: unknown;
}) {
	const engineError = extractEngineErrorBody(error);

	return (
		<InspectorGuardContext.Provider
			value={
				<Info>
					<Icon
						icon={faExclamationTriangle}
						className="text-4xl text-destructive"
					/>
					<p className="font-semibold">
						Unable to connect to the Inspector.
					</p>
					{engineError ? (
						<EngineErrorBlock body={engineError} />
					) : (
						<>
							<p className="text-sm text-muted-foreground">
								Ensure the Inspector is enabled and that you are
								running RivetKit{" "}
								<span className="font-mono-console">
									{RIVET_KIT_MIN_VERSION}
								</span>{" "}
								or newer.
							</p>
							{error ? (
								<ErrorDetails
									error={
										isRivetApiError(error)
											? error.body
											: error
									}
								/>
							) : null}
						</>
					)}
				</Info>
			}
		>
			{children}
		</InspectorGuardContext.Provider>
	);
}

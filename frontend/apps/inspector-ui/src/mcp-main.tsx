import { App } from "@modelcontextprotocol/ext-apps";
import {
	faDownLeftAndUpRightToCenter,
	faUpRightAndDownLeftFromCenter,
	Icon,
} from "@rivet-gg/icons";
import * as Sentry from "@sentry/react";
import { useCallback, useEffect, useState } from "react";
import ReactDOM from "react-dom/client";
import type { ActorId } from "@/components/actors/queries";
import { WithTooltip } from "@/components/ui/tooltip";
import "@/index.css";
import {
	describeFailure,
	failureSummary,
	type InspectorFailure,
	toolResultError,
} from "./mcp-error";
import { McpErrorPanel } from "./mcp-error-panel";
import { InspectorApp } from "./main";
import { initMcpTelemetry, readMcpAppTelemetry } from "./telemetry";

type ActorTarget =
	| { actorId: string }
	| { name: string; key?: string[]; method: "get"; skipReadyWait?: boolean }
	| {
			name: string;
			key?: string[];
			method: "getOrCreate";
			pool: string;
			input?: unknown;
			region?: string;
			crashPolicy?: "restart" | "sleep" | "destroy";
			skipReadyWait?: boolean;
	  };

type InspectorGrant = {
	token: string;
	proxyUrl: string;
	expiresAt: string;
	actorId: string;
	dashboardUrl?: string;
};

// Passing options replaces the SDK default `{ autoResize: true }`, so the
// resize notifications have to be re-enabled explicitly here.
const app = new App(
	{ name: "Rivet Actor Inspector", version: "0.1.0" },
	{ availableDisplayModes: ["inline", "fullscreen"] },
	{ strict: true, autoResize: true },
);

let currentActor: ActorTarget | undefined;
let currentGrant: InspectorGrant | undefined;

function structuredGrant(
	result: Awaited<ReturnType<typeof app.callServerTool>>,
	fallback: string,
): InspectorGrant {
	if (result.isError || !result.structuredContent) {
		throw toolResultError(result, fallback);
	}
	const value = result.structuredContent as Record<string, unknown>;
	for (const key of ["token", "proxyUrl", "expiresAt", "actorId"] as const) {
		if (typeof value[key] !== "string")
			throw new Error("The Inspector session response was malformed");
	}
	return value as InspectorGrant;
}

async function createSession(actor: ActorTarget): Promise<InspectorGrant> {
	return structuredGrant(
		await app.callServerTool({
			name: "rivet.ui.actor.session.create",
			arguments: { actor },
		}),
		"The temporary Inspector session could not be created",
	);
}

async function renewSession(token: string): Promise<InspectorGrant> {
	return structuredGrant(
		await app.callServerTool({
			name: "rivet.ui.actor.session.renew",
			arguments: { token },
		}),
		"The temporary Inspector session could not be renewed",
	);
}

async function revokeSession(token: string): Promise<void> {
	await app.callServerTool({
		name: "rivet.ui.actor.session.revoke",
		arguments: { token },
	});
}

// `create` mints a new session record rather than rotating the current one, so
// the grant it replaces stays valid until its own TTL and keeps counting
// against the per-principal session limit. `renew` rotates in place and needs
// no revocation. Hosts may fire tool results back to back, so swaps are
// serialized to keep a concurrent pair from both reading the same outgoing
// grant and leaking one of them.
let sessionSwap: Promise<unknown> = Promise.resolve();

function replaceSession(actor: ActorTarget): Promise<InspectorGrant> {
	const swap = sessionSwap.then(async () => {
		const superseded = currentGrant;
		const next = await createSession(actor);
		currentGrant = next;
		if (superseded)
			await revokeSession(superseded.token).catch((error) =>
				Sentry.captureException(error),
			);
		return next;
	});
	sessionSwap = swap.catch(() => {});
	return swap;
}

// Reporting the degraded state costs no turn, so the model can explain the
// panel if the user asks about it without the app forcing a reply.
function reportFailure(failure: InspectorFailure) {
	if (!app.getHostCapabilities()?.updateModelContext) return;
	void app
		.updateModelContext({
			content: [{ type: "text", text: failureSummary(failure) }],
		})
		.catch(() => {});
}

function McpInspector() {
	const [grant, setGrant] = useState<InspectorGrant>();
	const [failure, setFailure] = useState<InspectorFailure>();
	const [retrying, setRetrying] = useState(false);
	const [asked, setAsked] = useState(false);
	const [displayMode, setDisplayMode] = useState<"inline" | "fullscreen">(
		"inline",
	);

	const fail = useCallback((error: unknown, title: string) => {
		Sentry.captureException(error);
		const described = describeFailure(error, title);
		setFailure(described);
		setAsked(false);
		reportFailure(described);
	}, []);

	const openSession = useCallback(
		(actor: ActorTarget, title: string) => {
			setRetrying(true);
			void replaceSession(actor)
				.then((next) => {
					setGrant(next);
					setFailure(undefined);
				})
				.catch((error) => fail(error, title))
				.finally(() => setRetrying(false));
		},
		[fail],
	);

	useEffect(() => {
		const receiveInput = (params: {
			arguments?: Record<string, unknown>;
		}) => {
			const actor = params.arguments?.actor;
			if (actor && typeof actor === "object")
				currentActor = actor as ActorTarget;
		};
		const receiveResult = () => {
			if (!currentActor) return;
			openSession(currentActor, "Could not open the Inspector");
		};
		app.addEventListener("toolinput", receiveInput);
		app.addEventListener("toolresult", receiveResult);
		app.onhostcontextchanged = (context) => {
			document.documentElement.classList.toggle(
				"dark",
				context.theme !== "light",
			);
		};
		app.onteardown = async () => {
			if (currentGrant) await revokeSession(currentGrant.token);
			return {};
		};
		void app
			.connect()
			.catch((error) =>
				fail(error, "This host could not start the MCP App"),
			);
		return () => {
			app.removeEventListener("toolinput", receiveInput);
			app.removeEventListener("toolresult", receiveResult);
		};
	}, [fail, openSession]);

	useEffect(() => {
		if (!grant) return;
		const renewAt = Math.max(
			1_000,
			new Date(grant.expiresAt).getTime() - Date.now() - 30_000,
		);
		const timer = window.setTimeout(() => {
			void renewSession(grant.token)
				.then((next) => {
					currentGrant = next;
					setGrant(next);
				})
				.catch((error) => fail(error, "The Inspector session expired"));
		}, renewAt);
		return () => window.clearTimeout(timer);
	}, [grant, fail]);

	const retry = useCallback(() => {
		if (!currentActor) return;
		openSession(currentActor, "Could not reopen the Inspector");
	}, [openSession]);

	const ask = useCallback(() => {
		if (!failure) return;
		setAsked(true);
		void app
			.sendMessage({
				role: "user",
				content: [
					{
						type: "text",
						text: `${failureSummary(failure)}\n\nWhat should I do to get the Inspector working?`,
					},
				],
			})
			.catch(() => setAsked(false));
	}, [failure]);

	const toggleDisplayMode = useCallback(() => {
		const next = displayMode === "fullscreen" ? "inline" : "fullscreen";
		void app
			.requestDisplayMode({ mode: next })
			.then((result) =>
				setDisplayMode(
					result.mode === "fullscreen" ? "fullscreen" : "inline",
				),
			)
			.catch((error) => Sentry.captureException(error));
	}, [displayMode]);

	if (failure)
		return (
			<McpErrorPanel
				failure={failure}
				retrying={retrying}
				onRetry={
					failure.recoverable && currentActor ? retry : undefined
				}
				onAsk={app.getHostCapabilities()?.message ? ask : undefined}
				asked={asked}
			/>
		);
	if (!grant)
		return (
			<div className="flex h-full min-h-0 items-center justify-center p-6">
				<p className="text-sm text-muted-foreground">
					Connecting to the Rivet Actor Inspector…
				</p>
			</div>
		);
	const expanded = displayMode === "fullscreen";
	return (
		<div className="flex h-full min-h-[38rem] flex-col">
			{grant.dashboardUrl ? (
				<div className="shrink-0 border-b px-3 py-2 text-xs text-muted-foreground">
					Console and custom tabs are available in the{" "}
					<a
						className="font-medium text-foreground underline underline-offset-2"
						href={grant.dashboardUrl}
						target="_blank"
						rel="noreferrer"
					>
						full Rivet Inspector
					</a>
					.
				</div>
			) : null}
			<div className="min-h-0 flex-1">
				<InspectorApp
					key={grant.token}
					actorId={grant.actorId as ActorId}
					credentials={{
						url: grant.proxyUrl,
						inspectorToken: grant.token,
						token: grant.token,
					}}
					activeTab={undefined}
					standalone
					toolbar={
						<WithTooltip
							content={expanded ? "Collapse" : "Expand"}
							trigger={
								<button
									type="button"
									aria-label={
										expanded ? "Collapse" : "Expand"
									}
									className="rounded px-2 py-1.5 text-sm text-muted-foreground hover:bg-muted/60 hover:text-foreground"
									onClick={toggleDisplayMode}
								>
									<Icon
										icon={
											expanded
												? faDownLeftAndUpRightToCenter
												: faUpRightAndDownLeftFromCenter
										}
										className="size-3.5"
									/>
								</button>
							}
						/>
					}
				/>
			</div>
		</div>
	);
}

const root = document.getElementById("root");
if (!root) throw new Error("Inspector UI: #root element missing");
const reactRoot = ReactDOM.createRoot(root);
void initMcpTelemetry(readMcpAppTelemetry()).then((TelemetryProvider) => {
	reactRoot.render(
		<TelemetryProvider>
			<McpInspector />
		</TelemetryProvider>,
	);
});

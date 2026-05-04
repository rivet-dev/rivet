import { createRivetKit } from "@rivetkit/react";
import { createClient } from "rivetkit/client";
import mermaid from "mermaid";
import { Highlight, themes } from "prism-react-renderer";
import {
	Code,
	Compass,
	Clipboard,
	Database,
	FlaskConical,
	GitBranch,
	Globe,
	List,
	Network,
	Radio,
	RefreshCw,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { registry } from "../src/index.ts";
import {
	ACTION_TEMPLATES,
	type ActionTemplate,
	PAGE_GROUPS,
	PAGE_INDEX,
	type PageConfig,
} from "./page-data.ts";

const GROUP_ICONS: Record<string, React.ComponentType<{ size?: number }>> = {
	compass: Compass,
	code: Code,
	database: Database,
	radio: Radio,
	globe: Globe,
	"refresh-cw": RefreshCw,
	list: List,
	"git-branch": GitBranch,
	network: Network,
	"flask-conical": FlaskConical,
};

mermaid.initialize({
	startOnLoad: false,
	theme: "dark",
	themeVariables: {
		darkMode: true,
		background: "#0a0a0a",
		primaryColor: "#1c1c1e",
		primaryTextColor: "#ffffff",
		primaryBorderColor: "#3a3a3c",
		lineColor: "#3a3a3c",
		secondaryColor: "#2c2c2e",
		tertiaryColor: "#0f0f0f",
		fontFamily:
			"-apple-system, BlinkMacSystemFont, 'Segoe UI', 'Inter', Roboto, sans-serif",
		fontSize: "13px",
	},
});

function MermaidDiagram({ chart }: { chart: string }) {
	const ref = useRef<HTMLDivElement>(null);
	const [svg, setSvg] = useState("");

	useEffect(() => {
		let cancelled = false;
		const id = `mermaid-${Math.random().toString(36).slice(2)}`;
		mermaid.render(id, chart).then(({ svg: renderedSvg }) => {
			if (!cancelled) setSvg(renderedSvg);
		});
		return () => {
			cancelled = true;
		};
	}, [chart]);

	return (
		<div
			ref={ref}
			className="mermaid-diagram"
			dangerouslySetInnerHTML={{ __html: svg }}
		/>
	);
}

const viteRivetEndpoint = import.meta.env.VITE_RIVET_ENDPOINT as
	| string
	| undefined;
const rivetEndpoint =
	viteRivetEndpoint ?? `${globalThis.location.origin}/api/rivet`;
const mockAgenticLoopEndpoint =
	viteRivetEndpoint ?? "http://127.0.0.1:6420";
const mockAgenticLoopEndpointStorageKey = `kitchen-sink:mock-agentic-loop:endpoint:${mockAgenticLoopEndpoint}`;

const { useActor } = createRivetKit<typeof registry>(rivetEndpoint);

type ActorPanelActor = {
	connStatus: string | null;
	error: unknown;
	handle: {
		action: (request: { name: string; args: unknown[] }) => Promise<unknown>;
		fetch: (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>;
		webSocket: () => Promise<WebSocket>;
	} | null;
	connection: {
		on: (event: string, callback: (...args: unknown[]) => void) => () => void;
	} | null;
};

type LooseActorHook = (options: {
	name: string;
	key: string | string[];
	params?: Record<string, string>;
	createWithInput?: unknown;
}) => ActorPanelActor;

const useActorLoose = useActor as unknown as LooseActorHook;

type JsonResult<T> = { ok: true; value: T } | { ok: false; error: string };

function parseJson<T>(value: string): JsonResult<T> {
	try {
		const parsed = JSON.parse(value) as T;
		return { ok: true, value: parsed };
	} catch (error) {
		return {
			ok: false,
			error: error instanceof Error ? error.message : "Invalid JSON",
		};
	}
}

function parseKey(value: string): JsonResult<string | string[]> {
	const trimmed = value.trim();
	if (trimmed.startsWith("[")) {
		return parseJson<string[]>(trimmed);
	}
	return { ok: true, value: trimmed || "demo" };
}

function formatJson(value: unknown) {
	return JSON.stringify(value, null, 2);
}

function usePersistedState<T>(key: string, initial: T) {
	const [state, setState] = useState<T>(() => {
		const stored = localStorage.getItem(key);
		return stored ? (JSON.parse(stored) as T) : initial;
	});

	useEffect(() => {
		localStorage.setItem(key, JSON.stringify(state));
	}, [key, state]);

	return [state, setState] as const;
}

function resolvePage(pageId: string) {
	return PAGE_INDEX.find((page) => page.id === pageId) ?? PAGE_INDEX[0];
}

function formatActorName(name: string) {
	return name
		.replace(/([a-z0-9])([A-Z])/g, "$1 $2")
		.replace(/-/g, " ")
		.replace(/_/g, " ")
		.replace(/^\w/, (char) => char.toUpperCase());
}

function getStateAction(actorName: string): string | undefined {
	const templates = ACTION_TEMPLATES[actorName] ?? [];
	return templates.find((t) => t.args.length === 0)?.action;
}

// ── Main App ──────────────────────────────────────

export function App() {
	const [activePageId, setActivePageId] = usePersistedState(
		"kitchen-sink:page",
		PAGE_GROUPS[0].pages[0].id,
	);
	const activePage = resolvePage(activePageId);

	return (
		<div className="app">
			<aside className="sidebar">
				<div>
					<h1>Kitchen Sink</h1>
					<p className="subtitle">Explore every Rivet Actor feature</p>
				</div>

				<div className="mobile-select">
					<label htmlFor="page-select">Page</label>
					<select
						id="page-select"
						value={activePage.id}
						onChange={(event) => setActivePageId(event.target.value)}
					>
						{PAGE_GROUPS.map((group) => (
							<optgroup key={group.id} label={group.title}>
								{group.pages.map((page) => (
									<option key={page.id} value={page.id}>
										{page.title}
									</option>
								))}
							</optgroup>
						))}
					</select>
				</div>

				{PAGE_GROUPS.map((group) => {
					const Icon = GROUP_ICONS[group.icon];
					return (
						<div className="nav-group" key={group.id}>
							<div className="nav-group-title">
								{Icon && <Icon size={14} />}
								{group.title}
							</div>
							{group.pages.map((page) => (
								<button
									key={page.id}
									className={`nav-button ${page.id === activePage.id ? "active" : ""}`}
									onClick={() => setActivePageId(page.id)}
									type="button"
								>
									{page.title}
								</button>
							))}
						</div>
					);
				})}
			</aside>

			<main className="content">
				<header className="page-header">
					<h2 className="page-title">{activePage.title}</h2>
					<p className="page-description">{activePage.description}</p>
				</header>

				{activePage.id === "welcome" ? (
					<WelcomePanel />
				) : (
					<DemoPanel page={activePage} />
				)}
			</main>
		</div>
	);
}

// ── Demo Panel Router ─────────────────────────────

function DemoPanel({ page }: { page: PageConfig }) {
	if (page.demo === "diagram") {
		return <DiagramPanel page={page} />;
	}
	if (page.demo === "mock-agentic-loop") {
		return <MockAgenticLoopPanel page={page} />;
	}
	if (page.actors.length === 0) {
		return <ConfigPlayground />;
	}
	if (page.demo === "raw-http") {
		return <RawHttpPanel page={page} />;
	}
	if (page.demo === "raw-websocket") {
		return <RawWebSocketPanel page={page} />;
	}
	return <ActorDemoPanel page={page} />;
}

// ── Actor Demo Panel (tabs + view + code) ─────────

function ActorDemoPanel({ page }: { page: PageConfig }) {
	const [selectedIdx, setSelectedIdx] = useState(0);

	useEffect(() => {
		setSelectedIdx(0);
	}, [page.id]);

	const actorName = page.actors[selectedIdx] ?? page.actors[0];

	return (
		<div className="demo-container">
			{page.actors.length > 1 && (
				<div className="actor-tabs">
					{page.actors.map((name, idx) => (
						<button
							key={name}
							className={`actor-tab ${idx === selectedIdx ? "active" : ""}`}
							onClick={() => setSelectedIdx(idx)}
							type="button"
						>
							{formatActorName(name)}
						</button>
					))}
				</div>
			)}

			<ActorView
				key={`${page.id}-${actorName}`}
				actorName={actorName}
				page={page}
			/>

			<div className="demo-code-bottom">
				<div className="demo-code-label">
					<span className="section-label">Source</span>
				</div>
				<CodeBlock code={page.snippet} />
			</div>
		</div>
	);
}

// ── Actor View (two-column: controls | inspector) ─

function ActorView({
	actorName,
	page,
}: {
	actorName: string;
	page: PageConfig;
}) {
	const [keyInput, setKeyInput] = usePersistedState(
		`kitchen-sink:${page.id}:${actorName}:key`,
		`demo-${page.id}`,
	);
	const [paramsInput, setParamsInput] = usePersistedState(
		`kitchen-sink:${page.id}:${actorName}:params`,
		"{}",
	);
	const [createInput, setCreateInput] = usePersistedState(
		`kitchen-sink:${page.id}:${actorName}:input`,
		"{}",
	);

	const parsedKey = useMemo(() => parseKey(keyInput), [keyInput]);
	const parsedParams = useMemo(
		() => parseJson<Record<string, string>>(paramsInput),
		[paramsInput],
	);
	const parsedInput = useMemo(
		() => parseJson<unknown>(createInput),
		[createInput],
	);

	const resolvedParams =
		parsedParams.ok && paramsInput.trim() !== "{}"
			? parsedParams.value
			: undefined;
	const resolvedInput =
		parsedInput.ok && createInput.trim() !== "{}"
			? parsedInput.value
			: undefined;

	const actor = useActorLoose({
		name: actorName,
		key: parsedKey.ok ? parsedKey.value : "demo",
		params: resolvedParams,
		createWithInput: resolvedInput,
	});

	const templates = ACTION_TEMPLATES[actorName] ?? [];
	const stateAction = page.noAutoState ? undefined : getStateAction(actorName);

	const [stateRefreshCounter, setStateRefreshCounter] = useState(0);
	const triggerStateRefresh = useCallback(
		() => setStateRefreshCounter((c) => c + 1),
		[],
	);

	return (
		<div className="actor-view">
			<div className="actor-view-header">
				<div className="actor-view-title-row">
					{page.actors.length === 1 && (
						<span className="actor-name">{formatActorName(actorName)}</span>
					)}
					<div className="status-pill">
						<span
							className={`status-dot ${actor.connStatus === "connected"
									? "connected"
									: actor.error
										? "error"
										: ""
								}`}
						/>
						<span>{actor.connStatus ?? "idle"}</span>
					</div>
				</div>
				<div className="connection-fields-compact">
					<div className="field-compact">
						<label>Key</label>
						<input
							value={keyInput}
							onChange={(e) => setKeyInput(e.target.value)}
							placeholder="demo"
						/>
					</div>
					<div className="field-compact">
						<label>Params</label>
						<input
							value={paramsInput}
							onChange={(e) => setParamsInput(e.target.value)}
							placeholder="{}"
						/>
					</div>
					<div className="field-compact">
						<label>Input</label>
						<input
							value={createInput}
							onChange={(e) => setCreateInput(e.target.value)}
							placeholder="{}"
						/>
					</div>
				</div>
			</div>

			<div className="actor-columns">
				<div className="actor-controls">
					<div className="panel-label">Actions</div>
					<ActionRunner
						actor={actor}
						templates={templates}
						onActionComplete={triggerStateRefresh}
					/>
				</div>

				<div className="actor-inspector">
					{stateAction && (
						<StatePanel
							actor={actor}
							stateAction={stateAction}
							refreshTrigger={stateRefreshCounter}
						/>
					)}
					<EventsPanel actor={actor} defaultEvents={page.defaultEvents} />
				</div>
			</div>
		</div>
	);
}

// ── State Panel ───────────────────────────────────

function StatePanel({
	actor,
	stateAction,
	refreshTrigger,
}: {
	actor: ActorPanelActor;
	stateAction: string;
	refreshTrigger: number;
}) {
	const [state, setState] = useState<string>("");
	const [isRefreshing, setIsRefreshing] = useState(false);
	const handleRef = useRef(actor.handle);
	handleRef.current = actor.handle;

	const refresh = useCallback(async () => {
		const handle = handleRef.current;
		if (!handle) return;
		setIsRefreshing(true);
		try {
			const result = await handle.action({
				name: stateAction,
				args: [],
			});
			setState(formatJson(result));
		} catch (err) {
			setState(`Error: ${err instanceof Error ? err.message : String(err)}`);
		} finally {
			setIsRefreshing(false);
		}
	}, [stateAction]);

	useEffect(() => {
		if (actor.connStatus === "connected") {
			refresh();
		}
	}, [actor.connStatus, refresh]);

	useEffect(() => {
		if (refreshTrigger > 0) {
			refresh();
		}
	}, [refreshTrigger, refresh]);

	return (
		<div className="inspector-section">
			<div className="inspector-section-header">
				<span className="inspector-label">State</span>
				<button
					className="inspector-action-btn"
					onClick={refresh}
					disabled={!actor.handle || isRefreshing}
					type="button"
					title="Refresh state"
				>
					{isRefreshing ? "\u00b7\u00b7\u00b7" : "\u21bb"}
				</button>
			</div>
			<div className="state-display">
				{actor.connStatus !== "connected"
					? "Connecting\u2026"
					: state || "Loading\u2026"}
			</div>
		</div>
	);
}

// ── Events Panel ──────────────────────────────────

function EventsPanel({
	actor,
	defaultEvents,
}: {
	actor: ActorPanelActor;
	defaultEvents?: string[];
}) {
	const [eventInput, setEventInput] = useState("");
	const [subscribedEvents, setSubscribedEvents] = useState<string[]>(
		defaultEvents ?? [],
	);
	const [events, setEvents] = useState<
		Array<{ time: string; name: string; data: string }>
	>([]);

	const subscribedKey = subscribedEvents.join(",");

	const addEvent = () => {
		const name = eventInput.trim();
		if (name && !subscribedEvents.includes(name)) {
			setSubscribedEvents((prev) => [...prev, name]);
		}
		setEventInput("");
	};

	const removeEvent = (name: string) => {
		setSubscribedEvents((prev) => prev.filter((e) => e !== name));
	};

	useEffect(() => {
		if (!actor.connection || !subscribedKey) return;

		const names = subscribedKey.split(",");
		const stops = names.map((name) =>
			actor.connection!.on(name, (...args: unknown[]) => {
				const now = new Date();
				const time = [now.getHours(), now.getMinutes(), now.getSeconds()]
					.map((n) => n.toString().padStart(2, "0"))
					.join(":");
				setEvents((prev) => [
					{ time, name, data: formatJson(args.length === 1 ? args[0] : args) },
					...prev.slice(0, 49),
				]);
			}),
		);

		return () => {
			for (const stop of stops) stop();
		};
	}, [actor.connection, subscribedKey]);

	return (
		<div className="inspector-section">
			<div className="inspector-section-header">
				<span className="inspector-label">Events</span>
				<div className="inspector-controls">
					<input
						value={eventInput}
						onChange={(e) => setEventInput(e.target.value)}
						onKeyDown={(e) => {
							if (e.key === "Enter") addEvent();
						}}
						placeholder="add event name"
						className="inspector-input"
					/>
					<button
						className="inspector-action-btn"
						onClick={addEvent}
						disabled={!eventInput.trim()}
						type="button"
					>
						+
					</button>
					{events.length > 0 && (
						<button
							className="inspector-action-btn"
							onClick={() => setEvents([])}
							type="button"
						>
							Clear
						</button>
					)}
				</div>
			</div>
			{subscribedEvents.length > 0 && (
				<div className="event-tags">
					{subscribedEvents.map((name) => (
						<span key={name} className="event-tag">
							{name}
							<button
								className="event-tag-remove"
								onClick={() => removeEvent(name)}
								type="button"
							>
								&times;
							</button>
						</span>
					))}
				</div>
			)}
			<div className="events-display">
				{events.length === 0 ? (
					<div className="inspector-empty">
						{subscribedEvents.length > 0
							? "Waiting for events\u2026"
							: "Add event names to listen"}
					</div>
				) : (
					events.map((entry, i) => (
						<div className="event-row" key={i}>
							<span className="event-time">{entry.time}</span>
							<span className="event-name">{entry.name}</span>
							<span className="event-data">{entry.data}</span>
						</div>
					))
				)}
			</div>
		</div>
	);
}

// ── Code Block ────────────────────────────────────

function CodeBlock({ code }: { code: string }) {
	return (
		<Highlight theme={themes.oneDark} code={code} language="tsx">
			{({ tokens, getLineProps, getTokenProps }) => (
				<pre className="code-block">
					{tokens.map((line, i) => (
						<div key={i} {...getLineProps({ line })}>
							{line.map((token, key) => (
								<span key={key} {...getTokenProps({ token })} />
							))}
						</div>
					))}
				</pre>
			)}
		</Highlight>
	);
}

// ── Action Runner ─────────────────────────────────

function ActionRunner({
	actor,
	templates,
	onActionComplete,
}: {
	actor: ActorPanelActor;
	templates: ActionTemplate[];
	onActionComplete?: () => void;
}) {
	const [selectedIdx, setSelectedIdx] = useState(0);
	const selectedTemplate = templates[selectedIdx];
	const [argsInput, setArgsInput] = useState(
		selectedTemplate ? JSON.stringify(selectedTemplate.args) : "[]",
	);
	const [result, setResult] = useState<string>("");
	const [error, setError] = useState<string>("");
	const [isRunning, setIsRunning] = useState(false);
	const [lastLatency, setLastLatency] = useState<number | null>(null);
	const [inflight, setInflight] = useState(0);

	useEffect(() => {
		setSelectedIdx(0);
		if (templates[0]) {
			setArgsInput(JSON.stringify(templates[0].args));
		} else {
			setArgsInput("[]");
		}
	}, [templates]);

	const selectTemplate = (idx: number) => {
		setSelectedIdx(idx);
		setArgsInput(JSON.stringify(templates[idx].args));
		setResult("");
		setError("");
	};

	const parsedArgs = useMemo(
		() => parseJson<unknown[]>(argsInput),
		[argsInput],
	);

	const runAction = () => {
		setError("");
		const actionName = selectedTemplate?.action;
		if (!actor.handle) {
			setError("Actor handle is not ready.");
			return;
		}
		if (!actionName) {
			setError("Select an action to run.");
			return;
		}
		if (!parsedArgs.ok) {
			setError(parsedArgs.error);
			return;
		}

		setInflight((n) => n + 1);
		const start = performance.now();
		actor.handle
			.action({
				name: actionName,
				args: parsedArgs.value,
			})
			.then((response) => {
				setLastLatency(performance.now() - start);
				setResult(formatJson(response));
				onActionComplete?.();
			})
			.catch((err) => {
				setError(err instanceof Error ? err.message : String(err));
			})
			.finally(() => {
				setInflight((n) => n - 1);
			});
	};

	if (templates.length === 0) {
		return (
			<div className="no-actions">No actions available for this actor.</div>
		);
	}

	return (
		<div className="action-runner">
			<div className="segmented-control">
				{templates.map((template, idx) => (
					<button
						key={template.label}
						className={`segment ${idx === selectedIdx ? "active" : ""}`}
						onClick={() => selectTemplate(idx)}
						type="button"
					>
						{template.label}
					</button>
				))}
			</div>

			<div className="action-body">
				<div className="action-input-row">
					<input
						value={argsInput}
						onChange={(event) => setArgsInput(event.target.value)}
						placeholder="[]"
						className="action-args-input"
					/>
					<button
						className="primary"
						onClick={runAction}
						disabled={!actor.handle}
						type="button"
					>
						Run{inflight > 0 ? ` (${inflight})` : ""}
					</button>
				</div>
				{lastLatency !== null && (
					<div className="action-latency">{lastLatency.toFixed(0)}ms</div>
				)}
				{!parsedArgs.ok && <div className="notice">{parsedArgs.error}</div>}
				{error && <div className="notice">{error}</div>}
				{result && <pre className="action-result">{result}</pre>}
			</div>
		</div>
	);
}

type AgenticEntry = {
	request_id: string;
	idx: number;
	created_at: number;
};

type AgenticVerification = {
	requestId: string;
	expectedSeconds: number;
	count: number;
	contiguous?: boolean;
	missing?: number[];
	indexes: number[];
	ok?: boolean;
};

type AgenticHistory = {
	type: "history";
	totalRows: number;
	entries: AgenticEntry[];
	timestamp: number;
};

type AgenticDebugEvent = {
	type: "debugEvent";
	eventId: string;
	name: string;
	actorId: string;
	connectionId: string | null;
	requestId: string | null;
	details: Record<string, unknown>;
	createdAt: number;
	replayed: boolean;
};

type AgenticServerMessage =
	| { type: "hello"; connectionId: string; timestamp: number }
	| AgenticHistory
	| AgenticDebugEvent
	| {
			type: "pong";
			probeId: string;
			sleepStarted: boolean;
			sleepStartedAt: number | null;
			timestamp: number;
	  }
	| { type: "started"; requestId: string; seconds: number; timestamp: number }
	| {
			type: "progress";
			requestId: string;
			idx: number;
			seconds: number;
			createdAt: number;
	  }
	| {
			type: "done";
			requestId: string;
			seconds: number;
			timestamp: number;
			verification: AgenticVerification;
	  }
	| (AgenticVerification & { type: "verified" })
	| { type: "error"; message: string; timestamp: number };

type AgenticRequest = {
	requestId: string;
	seconds: number;
};

type AgenticHandle = {
	resolve: () => Promise<string>;
	webSocket: (
		path?: string,
		protocols?: string | string[],
		options?: {
			gateway?: { bypassConnectable?: boolean };
		},
	) => Promise<WebSocket>;
	fetch: (
		input: string,
		init?: RequestInit & {
			gateway?: { bypassConnectable?: boolean };
		},
	) => Promise<Response>;
	verify: (
		requestId: string,
		expectedSeconds: number,
	) => Promise<AgenticVerification>;
	verifyAll: (expectedRequests: AgenticRequest[]) => Promise<{
		type: "verifiedAll";
		expectedRequests: number;
		expectedTotalRows: number;
		totalRows: number;
		unexpectedRequestIds: string[];
		requests: AgenticVerification[];
		ok: boolean;
	}>;
};

type ActiveAgenticRequest = {
	requestId: string;
	seconds: number;
	expectedIdx: number;
	received: number[];
	lastProgressAt: number;
	startedAt: number;
};

type AgenticLogEntry = {
	id: string;
	level: "ok" | "warn" | "error" | "info";
	message: string;
	time: string;
};

function randomAgenticKey() {
	return `manual-agentic-${new Date().toISOString()}-${crypto.randomUUID()}`;
}

function nowTime() {
	return new Date().toLocaleTimeString();
}

function appendEndpointPath(endpoint: string, path: string): URL {
	const url = new URL(endpoint);
	const prefix = url.pathname.replace(/\/$/, "");
	url.pathname = `${prefix}${path}`;
	url.search = "";
	url.hash = "";
	return url;
}

function waitForSocketOpen(socket: WebSocket, timeoutMs = 10_000) {
	if (socket.readyState === WebSocket.OPEN) return Promise.resolve();

	return new Promise<void>((resolve, reject) => {
		const timeout = setTimeout(() => {
			cleanup();
			reject(new Error(`websocket open timed out after ${timeoutMs}ms`));
		}, timeoutMs);
		const cleanup = () => {
			clearTimeout(timeout);
			socket.removeEventListener("open", onOpen);
			socket.removeEventListener("close", onClose);
			socket.removeEventListener("error", onError);
		};
		const onOpen = () => {
			cleanup();
			resolve();
		};
		const onClose = (event: CloseEvent) => {
			cleanup();
			reject(
				new Error(
					`websocket closed before open code=${event.code} reason=${event.reason}`,
				),
			);
		};
		const onError = () => {
			cleanup();
			reject(new Error("websocket open error"));
		};
		socket.addEventListener("open", onOpen, { once: true });
		socket.addEventListener("close", onClose, { once: true });
		socket.addEventListener("error", onError, { once: true });
	});
}

function validateAgenticRows(
	entries: AgenticEntry[],
	expectedRequests: AgenticRequest[],
	activeRequest?: ActiveAgenticRequest | null,
) {
	const expectedByRequest = new Map(
		expectedRequests.map((request) => [request.requestId, request.seconds]),
	);
	const rowsByRequest = new Map<string, AgenticEntry[]>();

	for (const entry of entries) {
		const rows = rowsByRequest.get(entry.request_id) ?? [];
		rows.push(entry);
		rowsByRequest.set(entry.request_id, rows);
	}

	const problems: string[] = [];
	for (const request of expectedRequests) {
		const rows = rowsByRequest.get(request.requestId) ?? [];
		const indexes = rows.map((row) => row.idx);
		const contiguous =
			rows.length === request.seconds &&
			indexes.every((idx, offset) => idx === offset + 1);
		if (!contiguous) {
			problems.push(
				`${request.requestId.slice(0, 8)} expected ${request.seconds}, got [${indexes.join(", ")}]`,
			);
		}
	}

	if (activeRequest) {
		const rows = rowsByRequest.get(activeRequest.requestId) ?? [];
		const indexes = rows.map((row) => row.idx);
		const contiguousPrefix = indexes.every(
			(idx, offset) => idx === offset + 1,
		);
		if (rows.length > activeRequest.seconds || !contiguousPrefix) {
			problems.push(
				`${activeRequest.requestId.slice(0, 8)} active request expected partial 1-${activeRequest.seconds}, got [${indexes.join(", ")}]`,
			);
		}
	}

	for (const requestId of rowsByRequest.keys()) {
		if (
			!expectedByRequest.has(requestId) &&
			requestId !== activeRequest?.requestId
		) {
			problems.push(`${requestId.slice(0, 8)} was not expected`);
		}
	}

	return {
		ok: problems.length === 0,
		problems,
			expectedRows: expectedRequests.reduce(
				(total, request) => total + request.seconds,
				0,
			) + (activeRequest?.received.length ?? 0),
		};
	}

function sleepStatusFromPayload(
	source: string,
	payload: { sleepStarted?: unknown; sleepStartedAt?: unknown },
) {
	if (typeof payload.sleepStarted !== "boolean") {
		throw new Error(`${source} missing boolean sleepStarted`);
	}
	if (payload.sleepStarted && typeof payload.sleepStartedAt !== "number") {
		throw new Error(`${source} missing numeric sleepStartedAt`);
	}
	if (!payload.sleepStarted && payload.sleepStartedAt !== null) {
		throw new Error(`${source} expected null sleepStartedAt before sleep`);
	}
	return {
		sleepStarted: payload.sleepStarted,
		sleepStartedAt: payload.sleepStartedAt,
	};
}

function formatDebugDetails(details: Record<string, unknown>) {
	const entries = Object.entries(details).filter(
		([, value]) => value !== undefined && value !== null,
	);
	if (entries.length === 0) return "";

	return ` ${entries
		.map(([key, value]) => `${key}=${String(value)}`)
		.join(" ")}`;
}

function formatAgenticDebugEvent(event: AgenticDebugEvent) {
	const actorTime = new Date(event.createdAt).toLocaleTimeString();
	const lagMs = Date.now() - event.createdAt;
	const connection = event.connectionId
		? ` conn=${event.connectionId.slice(0, 8)}`
		: "";
	const request = event.requestId
		? ` req=${event.requestId.slice(0, 8)}`
		: "";
	const replay = event.replayed ? " replay" : "";

	return `actor${replay} ${event.name} at ${actorTime} lagMs=${lagMs}${connection}${request}${formatDebugDetails(event.details)}`;
}

function MockAgenticLoopPanel({ page }: { page: PageConfig }) {
	const [endpoint, setEndpoint] = usePersistedState(
		mockAgenticLoopEndpointStorageKey,
		mockAgenticLoopEndpoint,
	);
	const [namespace, setNamespace] = usePersistedState(
		"kitchen-sink:mock-agentic-loop:namespace",
		"default",
	);
	const [token, setToken] = usePersistedState(
		"kitchen-sink:mock-agentic-loop:token",
		"dev",
	);
	const [key, setKey] = useState(randomAgenticKey);
	const [actorId, setActorId] = useState("");
	const [connectionStatus, setConnectionStatus] = useState("idle");
	const [seconds, setSeconds] = useState(16);
	const [progressMarginMs, setProgressMarginMs] = useState(8_000);
	const [currentRequest, setCurrentRequest] = useState<{
		requestId: string;
		seconds: number;
		received: number[];
	} | null>(null);
	const [expectedRequests, setExpectedRequests] = useState<AgenticRequest[]>([]);
	const [lastVerification, setLastVerification] = useState("No requests yet.");
	const [lastHistory, setLastHistory] = useState("No history loaded yet.");
	const [lastBypass, setLastBypass] = useState("No bypass requests yet.");
	const [isConnecting, setIsConnecting] = useState(false);
	const [isRunningInference, setIsRunningInference] = useState(false);
	const [stats, setStats] = useState({
		requests: 0,
		expectedRows: 0,
		actualRows: 0,
		reconnects: 0,
		maxReconnectMs: 0,
		sleepPosts: 0,
		sleepErrors: 0,
		bypassHttpOk: 0,
		bypassWsOk: 0,
		actorStopping: 0,
		sleepProofHttp: 0,
		sleepProofWs: 0,
		validationErrors: 0,
	});
	const [logs, setLogs] = useState<AgenticLogEntry[]>([]);
	const [eventLogCopied, setEventLogCopied] = useState(false);

	const handleRef = useRef<AgenticHandle | null>(null);
	const socketRef = useRef<WebSocket | null>(null);
	const expectedRequestsRef = useRef<AgenticRequest[]>([]);
	const activeRequestRef = useRef<ActiveAgenticRequest | null>(null);
	const reconnectTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
	const progressTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
	const copyResetTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
	const reconnectStartedAtRef = useRef<number | null>(null);
	const mainSocketCleanupRef = useRef<(() => void) | null>(null);
	const closedByUserRef = useRef(false);

	const addLog = useCallback(
		(level: AgenticLogEntry["level"], message: string) => {
			setLogs((prev) => [
				{
					id: crypto.randomUUID(),
					level,
					message,
					time: nowTime(),
				},
				...prev.slice(0, 159),
			]);
		},
		[],
	);

	const copyEventLog = useCallback(async () => {
		if (logs.length === 0) return;

		const text = logs
			.map((entry) => `${entry.time}\t${entry.level}\t${entry.message}`)
			.join("\n");
		await navigator.clipboard.writeText(text);
		setEventLogCopied(true);
		if (copyResetTimerRef.current) clearTimeout(copyResetTimerRef.current);
		copyResetTimerRef.current = setTimeout(() => {
			setEventLogCopied(false);
			copyResetTimerRef.current = null;
		}, 1500);
	}, [logs]);

	const clearProgressTimer = useCallback(() => {
		if (progressTimerRef.current) {
			clearTimeout(progressTimerRef.current);
			progressTimerRef.current = null;
		}
	}, []);

	const markValidationError = useCallback((message: string) => {
		setStats((prev) => ({
			...prev,
			validationErrors: prev.validationErrors + 1,
		}));
		setLastVerification(message);
		addLog("error", message);
	}, [addLog]);

	const scheduleProgressTimeout = useCallback(() => {
		clearProgressTimer();
		const active = activeRequestRef.current;
		if (!active) return;
		const timeoutMs = 1_000 + progressMarginMs;
		progressTimerRef.current = setTimeout(() => {
			const latest = activeRequestRef.current;
			if (!latest) return;
			markValidationError(
				`progress timeout for ${latest.requestId.slice(0, 8)} at idx=${latest.expectedIdx}`,
			);
		}, timeoutMs);
	}, [clearProgressTimer, markValidationError, progressMarginMs]);

	const resetSession = useCallback(() => {
		closedByUserRef.current = true;
		if (reconnectTimerRef.current) clearTimeout(reconnectTimerRef.current);
		clearProgressTimer();
		mainSocketCleanupRef.current?.();
		mainSocketCleanupRef.current = null;
		socketRef.current?.close(1000, "new actor");
		socketRef.current = null;
		handleRef.current = null;
		expectedRequestsRef.current = [];
		activeRequestRef.current = null;
		setKey(randomAgenticKey());
		setActorId("");
		setConnectionStatus("idle");
		setCurrentRequest(null);
		setExpectedRequests([]);
		setIsRunningInference(false);
		setLastVerification("No requests yet.");
		setLastHistory("No history loaded yet.");
		setLastBypass("No bypass requests yet.");
		setStats({
			requests: 0,
			expectedRows: 0,
			actualRows: 0,
			reconnects: 0,
			maxReconnectMs: 0,
			sleepPosts: 0,
			sleepErrors: 0,
			bypassHttpOk: 0,
			bypassWsOk: 0,
			actorStopping: 0,
			sleepProofHttp: 0,
			sleepProofWs: 0,
			validationErrors: 0,
		});
		setLogs([]);
	}, [clearProgressTimer]);

	const requestHistory = useCallback(() => {
		if (socketRef.current?.readyState !== WebSocket.OPEN) return;
		socketRef.current.send(JSON.stringify({ type: "history" }));
		addLog("info", "history requested");
	}, [addLog]);

	const verifyAll = useCallback(async () => {
		const handle = handleRef.current;
		if (!handle) return;
		const result = await handle.verifyAll(expectedRequestsRef.current);
		if (!result.ok) {
			markValidationError(`aggregate verification failed: ${formatJson(result)}`);
			return;
		}
		setStats((prev) => ({
			...prev,
			actualRows: result.totalRows,
			expectedRows: result.expectedTotalRows,
		}));
		addLog(
			"ok",
			`verified all requests=${result.expectedRequests} rows=${result.totalRows}`,
		);
	}, [addLog, markValidationError]);

	const handleHistory = useCallback((message: AgenticHistory) => {
		const validation = validateAgenticRows(
			message.entries,
			expectedRequestsRef.current,
			activeRequestRef.current,
		);
		setStats((prev) => ({
			...prev,
			actualRows: message.totalRows,
			expectedRows: validation.expectedRows,
			validationErrors: validation.ok
				? prev.validationErrors
				: prev.validationErrors + 1,
		}));
		if (validation.ok) {
			setLastHistory(
				`history ok: rows=${message.totalRows}, expected=${validation.expectedRows}`,
			);
			addLog("ok", `history rows=${message.totalRows}`);
		} else {
			const text = `history mismatch: ${validation.problems.join("; ")}`;
			setLastHistory(text);
			addLog("error", text);
		}
	}, [addLog]);

	const handleProgress = useCallback((message: Extract<AgenticServerMessage, { type: "progress" }>) => {
		const active = activeRequestRef.current;
		if (!active || active.requestId !== message.requestId) {
			markValidationError(`unexpected progress for ${message.requestId.slice(0, 8)}`);
			return;
		}
		const now = performance.now();
		const gapMs = now - active.lastProgressAt;
		if (message.idx !== active.expectedIdx) {
			markValidationError(
				`expected idx=${active.expectedIdx}, got idx=${message.idx}`,
			);
		}
		active.received.push(message.idx);
		active.expectedIdx += 1;
		active.lastProgressAt = now;
		setCurrentRequest({
			requestId: active.requestId,
			seconds: active.seconds,
			received: [...active.received],
		});
		addLog(
			"info",
			`progress ${message.idx}/${message.seconds} gapMs=${gapMs.toFixed(0)}`,
		);
		scheduleProgressTimeout();
	}, [addLog, markValidationError, scheduleProgressTimeout]);

	const handleDone = useCallback(async (message: Extract<AgenticServerMessage, { type: "done" }>) => {
		const active = activeRequestRef.current;
		clearProgressTimer();
		setIsRunningInference(false);
		activeRequestRef.current = null;

		if (!active || active.requestId !== message.requestId) {
			markValidationError(`unexpected done for ${message.requestId.slice(0, 8)}`);
			return;
		}

		const contiguous =
			active.received.length === active.seconds &&
			active.received.every((idx, offset) => idx === offset + 1);
		if (!contiguous || !message.verification.ok) {
			markValidationError(
				`done verification failed: stream=[${active.received.join(", ")}], actor=${formatJson(message.verification)}`,
			);
			return;
		}

		const handle = handleRef.current;
		if (handle) {
			const explicit = await handle.verify(active.requestId, active.seconds);
			const explicitOk =
				explicit.count === active.seconds &&
				explicit.indexes.every((idx, offset) => idx === offset + 1);
			if (!explicitOk) {
				markValidationError(
					`action verification failed: ${formatJson(explicit)}`,
				);
				return;
			}
		}

		const completed = {
			requestId: active.requestId,
			seconds: active.seconds,
		};
		expectedRequestsRef.current = [...expectedRequestsRef.current, completed];
		setExpectedRequests(expectedRequestsRef.current);
		setStats((prev) => ({
			...prev,
			requests: prev.requests + 1,
			expectedRows: prev.expectedRows + active.seconds,
		}));
		setLastVerification(
			`request ${active.requestId.slice(0, 8)} ok: ${active.seconds}/${active.seconds} rows`,
		);
		addLog(
			"ok",
			`done ${active.requestId.slice(0, 8)} rows=${active.seconds}`,
		);
		await verifyAll();
		requestHistory();
	}, [addLog, clearProgressTimer, markValidationError, requestHistory, verifyAll]);

	const onSocketMessage = useCallback((event: MessageEvent) => {
		if (typeof event.data !== "string") return;
		const message = JSON.parse(event.data) as AgenticServerMessage;
		if (message.type === "hello") {
			addLog("ok", `main ws hello ${message.connectionId.slice(0, 8)}`);
			return;
		}
		if (message.type === "history") {
			handleHistory(message);
			return;
		}
		if (message.type === "debugEvent") {
			const level =
				message.name === "onSleepStart" || message.name === "webSocketClose"
					? "warn"
					: "info";
			addLog(level, formatAgenticDebugEvent(message));
			return;
		}
		if (message.type === "started") {
			addLog("ok", `started ${message.requestId.slice(0, 8)} seconds=${message.seconds}`);
			return;
		}
		if (message.type === "progress") {
			handleProgress(message);
			return;
		}
		if (message.type === "done") {
			void handleDone(message);
			return;
		}
		if (message.type === "error") {
			markValidationError(`actor error: ${message.message}`);
		}
	}, [addLog, handleDone, handleHistory, handleProgress, markValidationError]);

	const connect = useCallback(async (countReconnect = false) => {
		if (isConnecting) return;
		setIsConnecting(true);
		setConnectionStatus("connecting");
		closedByUserRef.current = false;
		const startedAt = performance.now();

		try {
			const client = createClient<typeof registry>({
				endpoint,
				namespace,
				token,
				encoding: "json",
			});
			const handle = client.mockAgenticLoop.getOrCreate([key]) as AgenticHandle;
			handleRef.current = handle;
			const resolvedActorId = await handle.resolve();
			setActorId(resolvedActorId);

			const socket = await handle.webSocket();
			await waitForSocketOpen(socket);
			socketRef.current = socket;
			const onClose = (event: CloseEvent) => {
				if (socketRef.current === socket) socketRef.current = null;
				setConnectionStatus("closed");
				const closedLocally = closedByUserRef.current;
				addLog(
					closedLocally ? "info" : "warn",
					`${closedLocally ? "local" : "remote"} main ws close code=${event.code} reason=${event.reason}`,
				);
				if (!closedLocally) {
					reconnectStartedAtRef.current = performance.now();
					reconnectTimerRef.current = setTimeout(() => {
						void connect(true);
					}, 500);
				}
			};
			const onError = () => {
				addLog("error", "main ws error");
			};
			socket.addEventListener("message", onSocketMessage);
			socket.addEventListener("close", onClose);
			socket.addEventListener("error", onError);
			mainSocketCleanupRef.current = () => {
				socket.removeEventListener("message", onSocketMessage);
				socket.removeEventListener("close", onClose);
				socket.removeEventListener("error", onError);
			};

			const elapsedMs = performance.now() - startedAt;
			setConnectionStatus("connected");
			if (countReconnect || reconnectStartedAtRef.current !== null) {
				const reconnectMs = reconnectStartedAtRef.current === null
					? elapsedMs
					: performance.now() - reconnectStartedAtRef.current;
				reconnectStartedAtRef.current = null;
				setStats((prev) => ({
					...prev,
					reconnects: prev.reconnects + 1,
					maxReconnectMs: Math.max(prev.maxReconnectMs, reconnectMs),
				}));
				addLog("ok", `reconnected in ${reconnectMs.toFixed(0)}ms`);
			} else {
				addLog("ok", `connected actor=${resolvedActorId}`);
			}
			requestHistory();
		} catch (error) {
			const message = error instanceof Error ? error.message : String(error);
			setConnectionStatus("error");
			addLog("error", `connect failed: ${message}`);
		} finally {
			setIsConnecting(false);
		}
	}, [addLog, endpoint, isConnecting, key, namespace, onSocketMessage, requestHistory, token]);

	const disconnect = useCallback(() => {
		closedByUserRef.current = true;
		if (reconnectTimerRef.current) clearTimeout(reconnectTimerRef.current);
		clearProgressTimer();
		mainSocketCleanupRef.current?.();
		mainSocketCleanupRef.current = null;
		socketRef.current?.close(1000, "manual disconnect");
		socketRef.current = null;
		setConnectionStatus("closed");
		addLog("warn", "main ws disconnected by client");
	}, [addLog, clearProgressTimer]);

	const runInference = useCallback(() => {
		const socket = socketRef.current;
		if (!socket || socket.readyState !== WebSocket.OPEN) {
			addLog("error", "main websocket is not connected");
			return;
		}
		if (activeRequestRef.current) {
			addLog("warn", "inference already active");
			return;
		}
		const safeSeconds = Math.max(1, Math.floor(seconds));
		const requestId = crypto.randomUUID();
		activeRequestRef.current = {
			requestId,
			seconds: safeSeconds,
			expectedIdx: 1,
			received: [],
			lastProgressAt: performance.now(),
			startedAt: performance.now(),
		};
		setCurrentRequest({ requestId, seconds: safeSeconds, received: [] });
		setIsRunningInference(true);
		socket.send(JSON.stringify({ type: "infer", requestId, seconds: safeSeconds }));
		addLog("info", `infer ${requestId.slice(0, 8)} seconds=${safeSeconds}`);
		scheduleProgressTimeout();
	}, [addLog, scheduleProgressTimeout, seconds]);

	const forceSleep = useCallback(async () => {
		if (!actorId) {
			addLog("error", "resolve an actor before forcing sleep");
			return;
		}
		const url = appendEndpointPath(
			endpoint,
			`/actors/${encodeURIComponent(actorId)}/sleep`,
		);
		url.searchParams.set("namespace", namespace);
		setStats((prev) => ({ ...prev, sleepPosts: prev.sleepPosts + 1 }));
		addLog("warn", "sleep post sent");
		try {
			const response = await fetch(url, {
				method: "POST",
				headers: {
					Authorization: token ? `Bearer ${token}` : "",
					"content-type": "application/json",
				},
				body: "{}",
			});
			const text = await response.text();
			if (!response.ok) {
				setStats((prev) => ({ ...prev, sleepErrors: prev.sleepErrors + 1 }));
				addLog("error", `sleep ${response.status}: ${text}`);
				return;
			}
			addLog("ok", `sleep ${response.status}`);
		} catch (error) {
			setStats((prev) => ({ ...prev, sleepErrors: prev.sleepErrors + 1 }));
			addLog("error", `sleep failed: ${error instanceof Error ? error.message : String(error)}`);
		}
	}, [actorId, addLog, endpoint, namespace, token]);

	const noteActorStopping = useCallback((label: string, status: number, text: string) => {
		setStats((prev) => ({ ...prev, actorStopping: prev.actorStopping + 1 }));
		setLastBypass(`${label}: actor.stopping (${status})`);
		addLog("warn", `${label} actor.stopping ${text}`);
	}, [addLog]);

	const testHttpBypass = useCallback(async () => {
		const handle = handleRef.current;
		if (!handle) {
			addLog("error", "connect before testing bypass");
			return;
		}
		try {
			const response = await handle.fetch("/bypass", {
				gateway: { bypassConnectable: true },
			});
			const text = await response.text();
			if (!response.ok) {
				if (text.includes('"code":"stopping"')) {
					noteActorStopping("http bypass", response.status, text);
					return;
				}
				setLastBypass(`http bypass failed ${response.status}: ${text}`);
				addLog("error", `http bypass ${response.status}: ${text}`);
				return;
			}
			const payload = JSON.parse(text) as {
				type?: string;
				transport?: string;
				sleepStarted?: unknown;
				sleepStartedAt?: unknown;
			};
			const sleepStatus = sleepStatusFromPayload("http bypass", payload);
			if (payload.type !== "bypass" || payload.transport !== "http") {
				throw new Error(`unexpected body ${text}`);
			}
			setStats((prev) => ({
				...prev,
				bypassHttpOk: prev.bypassHttpOk + 1,
				sleepProofHttp:
					prev.sleepProofHttp + (sleepStatus.sleepStarted ? 1 : 0),
			}));
			setLastBypass(
				`http bypass ok: sleepStarted=${sleepStatus.sleepStarted}`,
			);
			addLog("ok", `http bypass sleepStarted=${sleepStatus.sleepStarted}`);
		} catch (error) {
			const message = error instanceof Error ? error.message : String(error);
			setLastBypass(`http bypass error: ${message}`);
			addLog("error", `http bypass error: ${message}`);
		}
	}, [addLog, noteActorStopping]);

	const testWebSocketBypass = useCallback(async () => {
		const handle = handleRef.current;
		if (!handle) {
			addLog("error", "connect before testing bypass");
			return;
		}
		const probeId = crypto.randomUUID();
		let socket: WebSocket | null = null;
		try {
			socket = await handle.webSocket("/bypass", undefined, {
				gateway: { bypassConnectable: true },
			});
			await waitForSocketOpen(socket);
			const result = await new Promise<Extract<AgenticServerMessage, { type: "pong" }>>(
				(resolve, reject) => {
					const timeout = setTimeout(() => {
						cleanup();
						reject(new Error("timed out waiting for bypass pong"));
					}, 10_000);
					const cleanup = () => {
						clearTimeout(timeout);
						socket?.removeEventListener("message", onMessage);
						socket?.removeEventListener("close", onClose);
						socket?.removeEventListener("error", onError);
					};
					const onMessage = (event: MessageEvent) => {
						if (typeof event.data !== "string") return;
						const message = JSON.parse(event.data) as AgenticServerMessage;
						if (message.type !== "pong" || message.probeId !== probeId) return;
						cleanup();
						resolve(message);
					};
					const onClose = (event: CloseEvent) => {
						cleanup();
						reject(
							new Error(`closed code=${event.code} reason=${event.reason}`),
						);
					};
					const onError = () => {
						cleanup();
						reject(new Error("websocket error"));
					};
					socket?.addEventListener("message", onMessage);
					socket?.addEventListener("close", onClose, { once: true });
					socket?.addEventListener("error", onError, { once: true });
					socket?.send(JSON.stringify({ type: "ping", probeId }));
				},
			);
			const sleepStatus = sleepStatusFromPayload("ws bypass", result);
			setStats((prev) => ({
				...prev,
				bypassWsOk: prev.bypassWsOk + 1,
				sleepProofWs: prev.sleepProofWs + (sleepStatus.sleepStarted ? 1 : 0),
			}));
			setLastBypass(`ws bypass ok: sleepStarted=${sleepStatus.sleepStarted}`);
			addLog("ok", `ws bypass sleepStarted=${sleepStatus.sleepStarted}`);
		} catch (error) {
			const message = error instanceof Error ? error.message : String(error);
			if (message.includes("actor.stopping") || message.includes("Server Error")) {
				setStats((prev) => ({ ...prev, actorStopping: prev.actorStopping + 1 }));
				setLastBypass(`ws bypass transient close: ${message}`);
				addLog("warn", `ws bypass transient close: ${message}`);
			} else {
				setLastBypass(`ws bypass error: ${message}`);
				addLog("error", `ws bypass error: ${message}`);
			}
		} finally {
			if (
				socket &&
				(socket.readyState === WebSocket.OPEN ||
					socket.readyState === WebSocket.CONNECTING)
			) {
				socket.close(1000, "bypass probe complete");
			}
		}
	}, [addLog]);

	useEffect(() => {
		return () => {
			if (reconnectTimerRef.current) clearTimeout(reconnectTimerRef.current);
			if (copyResetTimerRef.current) clearTimeout(copyResetTimerRef.current);
			clearProgressTimer();
			mainSocketCleanupRef.current?.();
			mainSocketCleanupRef.current = null;
		};
	}, [clearProgressTimer]);

	const currentIndexes = currentRequest?.received ?? [];
	const invariantStatus =
		stats.validationErrors === 0 ? "pass" : "fail";

	return (
		<div className="agentic-lab">
			<section className="agentic-panel">
				<div className="agentic-panel-header">
					<div>
						<h3 className="card-title">Mock Agentic Loop</h3>
						<p className="card-subtitle">
							Use one raw WebSocket stream, explicit actions, manual sleep, and
							gateway bypass calls against the same actor.
						</p>
					</div>
					<div className={`agentic-status ${connectionStatus}`}>
						{connectionStatus}
					</div>
				</div>

				<div className="agentic-grid">
					<div className="form-row">
						<label htmlFor="agentic-endpoint">Endpoint</label>
						<input
							id="agentic-endpoint"
							value={endpoint}
							onChange={(event) => setEndpoint(event.target.value)}
						/>
					</div>
					<div className="form-row">
						<label htmlFor="agentic-namespace">Namespace</label>
						<input
							id="agentic-namespace"
							value={namespace}
							onChange={(event) => setNamespace(event.target.value)}
						/>
					</div>
					<div className="form-row">
						<label htmlFor="agentic-token">Token</label>
						<input
							id="agentic-token"
							value={token}
							onChange={(event) => setToken(event.target.value)}
						/>
					</div>
				</div>

				<div className="agentic-session-row">
					<div>
						<div className="agentic-kicker">Key</div>
						<div className="agentic-mono">{key}</div>
					</div>
					<div>
						<div className="agentic-kicker">Actor ID</div>
						<div className="agentic-mono">{actorId || "not resolved"}</div>
					</div>
				</div>

				<div className="button-row">
					<button className="primary" onClick={() => void connect()} disabled={isConnecting} type="button">
						{isConnecting ? "Connecting..." : "Connect"}
					</button>
					<button className="secondary" onClick={disconnect} type="button">
						Disconnect
					</button>
					<button className="ghost" onClick={resetSession} type="button">
						New Actor
					</button>
				</div>
			</section>

			<section className="agentic-panel">
				<div className="agentic-panel-header">
					<h3 className="card-title">Inference</h3>
					<div className={`agentic-status ${invariantStatus}`}>
						{stats.validationErrors === 0 ? "valid" : "invalid"}
					</div>
				</div>
				<div className="agentic-grid compact">
					<div className="form-row">
						<label htmlFor="agentic-seconds">Seconds</label>
						<input
							id="agentic-seconds"
							type="number"
							min={1}
							max={120}
							value={seconds}
							onChange={(event) => setSeconds(Number(event.target.value))}
						/>
					</div>
					<div className="form-row">
						<label htmlFor="agentic-margin">Progress Margin Ms</label>
						<input
							id="agentic-margin"
							type="number"
							min={0}
							value={progressMarginMs}
							onChange={(event) => setProgressMarginMs(Number(event.target.value))}
						/>
					</div>
				</div>
				<div className="button-row">
					<button
						className="primary"
						onClick={runInference}
						disabled={isRunningInference || connectionStatus !== "connected"}
						type="button"
					>
						Run Inference
					</button>
					<button className="secondary" onClick={requestHistory} disabled={connectionStatus !== "connected"} type="button">
						Request History
					</button>
				</div>
				<div className="agentic-stream">
					{currentRequest ? (
						<>
							<div className="agentic-mono">
								{currentRequest.requestId.slice(0, 8)} received{" "}
								{currentIndexes.length}/{currentRequest.seconds}
							</div>
							<div className="agentic-indexes">
								{Array.from({ length: currentRequest.seconds }, (_, index) => {
									const idx = index + 1;
									const received = currentIndexes.includes(idx);
									return (
										<span
											key={idx}
											className={`agentic-index ${received ? "received" : ""}`}
										>
											{idx}
										</span>
									);
								})}
							</div>
						</>
					) : (
						<div className="agentic-empty">No active inference.</div>
					)}
				</div>
			</section>

			<section className="agentic-panel">
				<div className="agentic-panel-header">
					<h3 className="card-title">Sleep and Bypass</h3>
				</div>
				<div className="button-row">
					<button className="primary" onClick={() => void forceSleep()} disabled={!actorId} type="button">
						Force Sleep
					</button>
					<button className="secondary" onClick={() => void testHttpBypass()} disabled={!handleRef.current} type="button">
						Test HTTP Bypass
					</button>
					<button className="secondary" onClick={() => void testWebSocketBypass()} disabled={!handleRef.current} type="button">
						Test WS Bypass
					</button>
				</div>
				<div className="agentic-result">{lastBypass}</div>
			</section>

			<section className="agentic-panel">
				<div className="agentic-panel-header">
					<h3 className="card-title">Event Log</h3>
					<div className="agentic-header-actions">
						<button
							aria-label="Copy event log"
							className="ghost icon-button"
							disabled={logs.length === 0}
							onClick={copyEventLog}
							title="Copy event log"
							type="button"
						>
							<Clipboard size={15} />
							<span>{eventLogCopied ? "Copied" : "Copy"}</span>
						</button>
						<button className="ghost" onClick={() => setLogs([])} type="button">
							Clear
						</button>
					</div>
				</div>
				<div className="agentic-log">
					{logs.length === 0 ? (
						<div className="agentic-empty">No activity yet.</div>
					) : (
						logs.map((entry) => (
							<div className={`agentic-log-row ${entry.level}`} key={entry.id}>
								<span>{entry.time}</span>
								<span>{entry.message}</span>
							</div>
						))
					)}
				</div>
			</section>

			<section className="agentic-panel">
				<div className="agentic-panel-header">
					<h3 className="card-title">Validation</h3>
				</div>
				<div className="agentic-stat-grid">
					<AgenticStat label="Requests" value={stats.requests} />
					<AgenticStat label="Rows" value={`${stats.actualRows}/${stats.expectedRows}`} />
					<AgenticStat label="Reconnects" value={stats.reconnects} />
					<AgenticStat label="Max Reconnect" value={`${stats.maxReconnectMs.toFixed(0)}ms`} />
					<AgenticStat label="Sleep Posts" value={stats.sleepPosts} />
					<AgenticStat label="Sleep Errors" value={stats.sleepErrors} />
					<AgenticStat label="HTTP Bypass" value={stats.bypassHttpOk} />
					<AgenticStat label="WS Bypass" value={stats.bypassWsOk} />
					<AgenticStat label="Stopping" value={stats.actorStopping} />
					<AgenticStat label="HTTP Proof" value={stats.sleepProofHttp} />
					<AgenticStat label="WS Proof" value={stats.sleepProofWs} />
					<AgenticStat label="Validation Errors" value={stats.validationErrors} />
				</div>
				<div className="agentic-result">{lastVerification}</div>
				<div className="agentic-result">{lastHistory}</div>
			</section>

			<div className="demo-code-bottom">
				<div className="demo-code-label">
					<span className="section-label">Source</span>
				</div>
				<CodeBlock code={page.snippet} />
			</div>
		</div>
	);
}

function AgenticStat({
	label,
	value,
}: {
	label: string;
	value: string | number;
}) {
	return (
		<div className="agentic-stat">
			<span>{label}</span>
			<strong>{value}</strong>
		</div>
	);
}

// ── Welcome / Diagram / Config ────────────────────

function WelcomePanel() {
	return (
		<section className="card">
			<p className="card-subtitle">
				This kitchen sink lets you interact with every Rivet Actor feature in
				one place. Pick a topic from the sidebar to connect to live actors,
				invoke actions, and observe events in real time.
			</p>
		</section>
	);
}

function DiagramPanel({ page }: { page: PageConfig }) {
	if (!page.diagram) return null;

	return (
		<section className="card">
			<MermaidDiagram chart={page.diagram} />
		</section>
	);
}

function ConfigPlayground() {
	const [jsonInput, setJsonInput] = useState(
		'{\n  "key": ["demo"],\n  "params": {\n    "region": "local"\n  }\n}',
	);
	const parsed = parseJson<Record<string, unknown>>(jsonInput);

	return (
		<section className="card">
			<div>
				<h3 className="card-title">Configuration Playground</h3>
				<p className="card-subtitle">
					Edit JSON to explore how actor configuration payloads are shaped.
				</p>
			</div>
			<div className="demo-grid">
				<div className="form-row">
					<label htmlFor="config-json">Configuration JSON</label>
					<textarea
						id="config-json"
						value={jsonInput}
						onChange={(event) => setJsonInput(event.target.value)}
					/>
				</div>
				<div className="form-row">
					<div className="card-subtitle">Parsed Output</div>
					<div className="code-block">
						{parsed.ok ? formatJson(parsed.value) : parsed.error}
					</div>
				</div>
			</div>
		</section>
	);
}

// ── Raw HTTP Panel ────────────────────────────────

function RawHttpPanel({ page }: { page: PageConfig }) {
	const [selectedActor, setSelectedActor] = useState(page.actors[0] ?? "");
	const [path, setPath] = useState(page.rawHttpDefaults?.path ?? "/api/hello");
	const [method, setMethod] = useState(page.rawHttpDefaults?.method ?? "GET");
	const [body, setBody] = useState(page.rawHttpDefaults?.body ?? "");
	const [response, setResponse] = useState("");
	const [error, setError] = useState("");
	const [isLoading, setIsLoading] = useState(false);

	useEffect(() => {
		setSelectedActor(page.actors[0] ?? "");
		setPath(page.rawHttpDefaults?.path ?? "/api/hello");
		setMethod(page.rawHttpDefaults?.method ?? "GET");
		setBody(page.rawHttpDefaults?.body ?? "");
	}, [page.id, page.actors, page.rawHttpDefaults]);

	const actor = useActorLoose({
		name: selectedActor,
		key: ["demo"],
	});

	const sendRequest = async () => {
		setError("");
		setResponse("");
		if (!actor.handle) {
			setError("Actor handle is not ready.");
			return;
		}
		setIsLoading(true);
		try {
			const response = await actor.handle.fetch(path, {
				method,
				body: method === "GET" ? undefined : body || undefined,
				headers:
					method === "GET" ? undefined : { "Content-Type": "application/json" },
			});
			const text = await response.text();
			setResponse(`Status ${response.status}\n${text}`);
		} catch (err) {
			setError(err instanceof Error ? err.message : String(err));
		} finally {
			setIsLoading(false);
		}
	};

	return (
		<section className="card">
			<div>
				<h3 className="card-title">Raw HTTP Demo</h3>
				<p className="card-subtitle">
					Send raw HTTP requests to actor request handlers.
				</p>
			</div>
			<div className="demo-grid">
				<div className="form-row">
					<label htmlFor="raw-http-actor">Actor</label>
					<select
						id="raw-http-actor"
						value={selectedActor}
						onChange={(event) => setSelectedActor(event.target.value)}
					>
						{page.actors.map((actorName) => (
							<option key={actorName} value={actorName}>
								{formatActorName(actorName)}
							</option>
						))}
					</select>
				</div>
				<div className="form-row">
					<label htmlFor="raw-http-method">Method</label>
					<select
						id="raw-http-method"
						value={method}
						onChange={(event) => setMethod(event.target.value)}
					>
						{["GET", "POST", "PUT", "DELETE"].map((option) => (
							<option key={option} value={option}>
								{option}
							</option>
						))}
					</select>
				</div>
				<div className="form-row">
					<label htmlFor="raw-http-path">Path</label>
					<input
						id="raw-http-path"
						value={path}
						onChange={(event) => setPath(event.target.value)}
					/>
				</div>
				<div className="form-row">
					<label htmlFor="raw-http-body">Body</label>
					<textarea
						id="raw-http-body"
						value={body}
						onChange={(event) => setBody(event.target.value)}
						placeholder='{"count": 1}'
					/>
				</div>
			</div>
			<div className="button-row">
				<button
					className="primary"
					onClick={sendRequest}
					disabled={!actor.handle || isLoading}
					type="button"
				>
					{isLoading ? "Sending..." : "Send Request"}
				</button>
			</div>
			{error && <div className="notice">Error: {error}</div>}
			{response && <div className="code-block">{response}</div>}
		</section>
	);
}

// ── Raw WebSocket Panel ───────────────────────────

function RawWebSocketPanel({ page }: { page: PageConfig }) {
	const [selectedActor, setSelectedActor] = useState(page.actors[0] ?? "");
	const [message, setMessage] = useState('{"type": "ping"}');
	const [log, setLog] = useState<string[]>([]);
	const socketRef = useRef<WebSocket | null>(null);

	useEffect(() => {
		setSelectedActor(page.actors[0] ?? "");
	}, [page.id, page.actors]);

	const actor = useActorLoose({
		name: selectedActor,
		key: ["demo"],
	});

	const connect = async () => {
		if (!actor.handle) return;
		if (socketRef.current) return;
		const socket = await actor.handle.webSocket();
		socketRef.current = socket;
		setLog((prev) => ["connected", ...prev]);
		socket.addEventListener("message", (event: MessageEvent) => {
			setLog((prev) => [`message: ${event.data}`, ...prev]);
		});
		socket.addEventListener("close", () => {
			setLog((prev) => ["disconnected", ...prev]);
			socketRef.current = null;
		});
	};

	const disconnect = () => {
		socketRef.current?.close();
		socketRef.current = null;
	};

	const sendMessage = () => {
		if (!socketRef.current) return;
		socketRef.current.send(message);
		setLog((prev) => [`sent: ${message}`, ...prev]);
	};

	const sendBinary = () => {
		if (!socketRef.current) return;
		socketRef.current.send(new Uint8Array([1, 2, 3, 4]));
		setLog((prev) => ["sent: <binary 1,2,3,4>", ...prev]);
	};

	return (
		<section className="card">
			<div>
				<h3 className="card-title">Raw WebSocket Demo</h3>
				<p className="card-subtitle">
					Connect to a raw WebSocket handler and send messages.
				</p>
			</div>
			<div className="demo-grid">
				<div className="form-row">
					<label htmlFor="raw-ws-actor">Actor</label>
					<select
						id="raw-ws-actor"
						value={selectedActor}
						onChange={(event) => setSelectedActor(event.target.value)}
					>
						{page.actors.map((actorName) => (
							<option key={actorName} value={actorName}>
								{formatActorName(actorName)}
							</option>
						))}
					</select>
				</div>
				<div className="form-row">
					<label htmlFor="raw-ws-message">Message</label>
					<textarea
						id="raw-ws-message"
						value={message}
						onChange={(event) => setMessage(event.target.value)}
					/>
				</div>
			</div>
			<div className="button-row">
				<button
					className="primary"
					onClick={connect}
					disabled={!actor.handle || Boolean(socketRef.current)}
					type="button"
				>
					Connect
				</button>
				<button
					className="secondary"
					onClick={disconnect}
					disabled={!socketRef.current}
					type="button"
				>
					Disconnect
				</button>
				<button
					className="secondary"
					onClick={sendMessage}
					disabled={!socketRef.current}
					type="button"
				>
					Send Message
				</button>
				<button
					className="ghost"
					onClick={sendBinary}
					disabled={!socketRef.current}
					type="button"
				>
					Send Binary
				</button>
			</div>
			<div className="event-log">
				{log.length === 0
					? "No WebSocket activity yet."
					: log.map((entry, index) => (
						<div className="event-entry" key={`${entry}-${index}`}>
							{entry}
						</div>
					))}
			</div>
		</section>
	);
}

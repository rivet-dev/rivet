import { createRivetKit } from "@rivetkit/react";
import mermaid from "mermaid";
import { Highlight, themes } from "prism-react-renderer";
import {
	Code,
	Compass,
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
import type { registry } from "../src/actors.ts";
import {
	ACTION_TEMPLATES,
	type ActionTemplate,
	PAGE_GROUPS,
	PAGE_INDEX,
	type PageConfig,
} from "./page-data.ts";

type ActorName = (typeof registry)["config"]["use"] extends Record<infer K, unknown> ? K & string : never;

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
		fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI', 'Inter', Roboto, sans-serif",
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
		return () => { cancelled = true; };
	}, [chart]);

	return <div ref={ref} className="mermaid-diagram" dangerouslySetInnerHTML={{ __html: svg }} />;
}

const { useActor } = createRivetKit<typeof registry>(
	`${location.origin}/api/rivet`,
);

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
	return templates.find(t => t.args.length === 0)?.action;
}

// ── Main App ──────────────────────────────────────

export function App() {
	const [activePageId, setActivePageId] = usePersistedState(
		"sandbox:page",
		PAGE_GROUPS[0].pages[0].id,
	);
	const activePage = resolvePage(activePageId);

	return (
		<div className="app">
			<aside className="sidebar">
				<div>
					<h1>Actor Sandbox</h1>
					<p className="subtitle">
						Explore every Rivet Actor feature
					</p>
				</div>

				<div className="mobile-select">
					<label htmlFor="page-select">Page</label>
					<select
						id="page-select"
						value={activePage.id}
						onChange={(event) =>
							setActivePageId(event.target.value)
						}
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

	useEffect(() => { setSelectedIdx(0); }, [page.id]);

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

function ActorView({ actorName, page }: { actorName: string; page: PageConfig }) {
	const [keyInput, setKeyInput] = usePersistedState(
		`sandbox:${page.id}:${actorName}:key`,
		`demo-${page.id}`,
	);
	const [paramsInput, setParamsInput] = usePersistedState(
		`sandbox:${page.id}:${actorName}:params`,
		"{}",
	);
	const [createInput, setCreateInput] = usePersistedState(
		`sandbox:${page.id}:${actorName}:input`,
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

	const actor = useActor({
		name: actorName as ActorName,
		key: parsedKey.ok ? parsedKey.value : "demo",
		params: resolvedParams,
		createWithInput: resolvedInput,
	});

	const templates = ACTION_TEMPLATES[actorName] ?? [];
	const stateAction = getStateAction(actorName);

	const [stateRefreshCounter, setStateRefreshCounter] = useState(0);
	const triggerStateRefresh = useCallback(
		() => setStateRefreshCounter(c => c + 1),
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
							className={`status-dot ${
								actor.connStatus === "connected"
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
					<EventsPanel actor={actor} />
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
	actor: ReturnType<typeof useActor>;
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

function EventsPanel({ actor }: { actor: ReturnType<typeof useActor> }) {
	const [eventName, setEventName] = useState("");
	const [events, setEvents] = useState<Array<{ time: string; data: string }>>([]);

	useEffect(() => {
		if (!eventName.trim() || !actor.connection) return;

		const stop = actor.connection.on(eventName, (...args: unknown[]) => {
			const now = new Date();
			const time = [now.getHours(), now.getMinutes(), now.getSeconds()]
				.map(n => n.toString().padStart(2, "0"))
				.join(":");
			setEvents((prev) => [
				{ time, data: formatJson(args.length === 1 ? args[0] : args) },
				...prev.slice(0, 49),
			]);
		});

		return () => { stop(); };
	}, [actor.connection, eventName]);

	return (
		<div className="inspector-section">
			<div className="inspector-section-header">
				<span className="inspector-label">Events</span>
				<div className="inspector-controls">
					<input
						value={eventName}
						onChange={(e) => setEventName(e.target.value)}
						placeholder="event name"
						className="inspector-input"
					/>
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
			<div className="events-display">
				{events.length === 0 ? (
					<div className="inspector-empty">
						{eventName
							? "Waiting for events\u2026"
							: "Enter an event name to listen"}
					</div>
				) : (
					events.map((entry, i) => (
						<div className="event-row" key={i}>
							<span className="event-time">{entry.time}</span>
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
	actor: ReturnType<typeof useActor>;
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

	const runAction = async () => {
		setError("");
		setResult("");
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

		setIsRunning(true);
		try {
			const response = await actor.handle.action({
				name: actionName,
				args: parsedArgs.value,
			});
			setResult(formatJson(response));
			onActionComplete?.();
		} catch (err) {
			setError(err instanceof Error ? err.message : String(err));
		} finally {
			setIsRunning(false);
		}
	};

	if (templates.length === 0) {
		return <div className="no-actions">No actions available for this actor.</div>;
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
						disabled={!actor.handle || isRunning}
						type="button"
					>
						{isRunning ? "\u00b7\u00b7\u00b7" : "Run"}
					</button>
				</div>
				{!parsedArgs.ok && <div className="notice">{parsedArgs.error}</div>}
				{error && <div className="notice">{error}</div>}
				{result && <pre className="action-result">{result}</pre>}
			</div>
		</div>
	);
}

// ── Welcome / Diagram / Config ────────────────────

function WelcomePanel() {
	return (
		<section className="card">
			<p className="card-subtitle">
				This sandbox lets you interact with every Rivet Actor feature
				in one place. Pick a topic from the sidebar to connect to
				live actors, invoke actions, and observe events in real time.
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
					Edit JSON to explore how actor configuration payloads are
					shaped.
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
	const [path, setPath] = useState(
		page.rawHttpDefaults?.path ?? "/api/hello",
	);
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

	const actor = useActor({
		name: selectedActor as ActorName,
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
					method === "GET"
						? undefined
						: { "Content-Type": "application/json" },
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
						onChange={(event) =>
							setSelectedActor(event.target.value)
						}
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

	const actor = useActor({
		name: selectedActor as ActorName,
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
						onChange={(event) =>
							setSelectedActor(event.target.value)
						}
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
							<div
								className="event-entry"
								key={`${entry}-${index}`}
							>
								{entry}
							</div>
						))}
			</div>
		</section>
	);
}

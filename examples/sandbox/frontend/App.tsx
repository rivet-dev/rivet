import { createRivetKit } from "@rivetkit/react";
import { useEffect, useMemo, useRef, useState } from "react";
import type { registry } from "../src/actors.ts";
import {
	ACTION_TEMPLATES,
	type ActionTemplate,
	PAGE_GROUPS,
	PAGE_INDEX,
	type PageConfig,
} from "./page-data.ts";

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

				{PAGE_GROUPS.map((group) => (
					<div className="nav-group" key={group.id}>
						<div className="nav-group-title">{group.title}</div>
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
				))}
			</aside>

			<main className="content">
				<header className="page-header">
					<h2 className="page-title">{activePage.title}</h2>
					<p className="page-description">{activePage.description}</p>
					{activePage.docs.length > 0 && (
						<div className="doc-links">
							{activePage.docs.map((doc) => (
								<a
									key={doc.href}
									className="doc-link"
									href={doc.href}
									target="_blank"
									rel="noreferrer"
								>
									{doc.label}
								</a>
							))}
						</div>
					)}
				</header>

				<InteractivePanel page={activePage} />

				<section className="card">
					<div>
						<h3 className="card-title">Code Snippet</h3>
						<p className="card-subtitle">
							Use this snippet as a starting point for the actor
							features in this page.
						</p>
					</div>
					<pre className="code-block">{activePage.snippet}</pre>
				</section>
			</main>
		</div>
	);
}

function InteractivePanel({ page }: { page: PageConfig }) {
	if (page.actors.length === 0) {
		return <ConfigPlayground />;
	}

	if (page.demo === "raw-http") {
		return <RawHttpPanel page={page} />;
	}

	if (page.demo === "raw-websocket") {
		return <RawWebSocketPanel page={page} />;
	}

	return <ActionPanel page={page} />;
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

function ActionPanel({ page }: { page: PageConfig }) {
	const [selectedActor, setSelectedActor] = useState(page.actors[0] ?? "");

	useEffect(() => {
		setSelectedActor(page.actors[0] ?? "");
	}, [page.id, page.actors]);

	const [keyInput, setKeyInput] = usePersistedState(
		`sandbox:${page.id}:key`,
		`demo-${page.id}`,
	);
	const [paramsInput, setParamsInput] = usePersistedState(
		`sandbox:${page.id}:params`,
		"{}",
	);
	const [createInput, setCreateInput] = usePersistedState(
		`sandbox:${page.id}:input`,
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
		name: selectedActor as keyof typeof registry,
		key: parsedKey.ok ? parsedKey.value : "demo",
		params: resolvedParams,
		createWithInput: resolvedInput,
	});

	const templates = ACTION_TEMPLATES[selectedActor] ?? [];

	return (
		<section className="card">
			<div>
				<h3 className="card-title">Interactive Demo</h3>
				<p className="card-subtitle">
					Connect to an actor and invoke actions with custom payloads.
				</p>
			</div>
			<div className="demo-grid">
				<div className="form-row">
					<label htmlFor="actor-select">Actor</label>
					<select
						id="actor-select"
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
					<label htmlFor="actor-key">Actor Key</label>
					<input
						id="actor-key"
						value={keyInput}
						onChange={(event) => setKeyInput(event.target.value)}
						placeholder='"demo" or ["team", "alpha"]'
					/>
					{!parsedKey.ok && (
						<div className="notice">
							Key JSON error: {parsedKey.error}
						</div>
					)}
				</div>
				<div className="form-row">
					<label htmlFor="actor-params">
						Connection Params (JSON)
					</label>
					<textarea
						id="actor-params"
						value={paramsInput}
						onChange={(event) => setParamsInput(event.target.value)}
					/>
					{!parsedParams.ok && (
						<div className="notice">
							Params JSON error: {parsedParams.error}
						</div>
					)}
				</div>
				<div className="form-row">
					<label htmlFor="actor-input">Create Input (JSON)</label>
					<textarea
						id="actor-input"
						value={createInput}
						onChange={(event) => setCreateInput(event.target.value)}
					/>
					{!parsedInput.ok && (
						<div className="notice">
							Input JSON error: {parsedInput.error}
						</div>
					)}
				</div>
			</div>

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

			<ActionRunner actor={actor} templates={templates} />
			<EventListener actor={actor} />
		</section>
	);
}

function ActionRunner({
	actor,
	templates,
}: {
	actor: ReturnType<typeof useActor>;
	templates: ActionTemplate[];
}) {
	const initialTemplate = templates[0];
	const [actionName, setActionName] = useState(initialTemplate?.action ?? "");
	const [argsInput, setArgsInput] = useState(
		initialTemplate ? formatJson(initialTemplate.args) : "[]",
	);
	const [result, setResult] = useState<string>("");
	const [error, setError] = useState<string>("");
	const [isRunning, setIsRunning] = useState(false);

	useEffect(() => {
		if (!initialTemplate) {
			setActionName("");
			setArgsInput("[]");
			return;
		}
		setActionName(initialTemplate.action);
		setArgsInput(formatJson(initialTemplate.args));
	}, [initialTemplate]);

	const parsedArgs = useMemo(
		() => parseJson<unknown[]>(argsInput),
		[argsInput],
	);

	const runAction = async () => {
		setError("");
		setResult("");
		if (!actor.handle) {
			setError("Actor handle is not ready.");
			return;
		}
		if (!actionName.trim()) {
			setError("Enter an action name to run.");
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
		} catch (err) {
			setError(err instanceof Error ? err.message : String(err));
		} finally {
			setIsRunning(false);
		}
	};

	const applyTemplate = (template: ActionTemplate) => {
		setActionName(template.action);
		setArgsInput(formatJson(template.args));
	};

	return (
		<div className="card">
			<div>
				<h4 className="card-title">Actions</h4>
				<p className="card-subtitle">
					Call an action by name and pass JSON arguments.
				</p>
			</div>
			{templates.length > 0 && (
				<div className="button-row">
					{templates.map((template) => (
						<button
							key={template.label}
							onClick={() => applyTemplate(template)}
							className="secondary"
							type="button"
						>
							{template.label}
						</button>
					))}
				</div>
			)}
			<div className="demo-grid">
				<div className="form-row">
					<label htmlFor="action-name">Action Name</label>
					<input
						id="action-name"
						value={actionName}
						onChange={(event) => setActionName(event.target.value)}
						placeholder="increment"
					/>
				</div>
				<div className="form-row">
					<label htmlFor="action-args">Args (JSON array)</label>
					<textarea
						id="action-args"
						value={argsInput}
						onChange={(event) => setArgsInput(event.target.value)}
					/>
					{!parsedArgs.ok && (
						<div className="notice">
							Args JSON error: {parsedArgs.error}
						</div>
					)}
				</div>
			</div>
			<div className="button-row">
				<button
					className="primary"
					onClick={runAction}
					disabled={!actor.handle || isRunning}
					type="button"
				>
					{isRunning ? "Running..." : "Run Action"}
				</button>
				<button
					className="secondary"
					onClick={() => {
						setResult("");
						setError("");
					}}
					type="button"
				>
					Clear Output
				</button>
			</div>
			{error && <div className="notice">Error: {error}</div>}
			{result && <div className="code-block">{result}</div>}
		</div>
	);
}

function EventListener({ actor }: { actor: ReturnType<typeof useActor> }) {
	const [eventName, setEventName] = useState("newCount");
	const [isListening, setIsListening] = useState(false);
	const [events, setEvents] = useState<string[]>([]);

	useEffect(() => {
		if (!isListening || !eventName.trim() || !actor.connection) return;

		const stop = actor.connection.on(eventName, (...args: unknown[]) => {
			setEvents((prev) => [
				`${eventName}: ${formatJson(args.length === 1 ? args[0] : args)}`,
				...prev,
			]);
		});

		return () => {
			stop();
		};
	}, [actor.connection, eventName, isListening]);

	return (
		<div className="card">
			<div>
				<h4 className="card-title">Events</h4>
				<p className="card-subtitle">
					Listen for broadcast events while the connection is active.
				</p>
			</div>
			<div className="demo-grid">
				<div className="form-row">
					<label htmlFor="event-name">Event Name</label>
					<input
						id="event-name"
						value={eventName}
						onChange={(event) => setEventName(event.target.value)}
						placeholder="newCount"
					/>
				</div>
				<div className="form-row">
					<div className="card-subtitle">Listening</div>
					<div className="button-row">
						<button
							className={isListening ? "primary" : "secondary"}
							onClick={() => setIsListening((prev) => !prev)}
							type="button"
						>
							{isListening ? "Stop" : "Start"}
						</button>
						<button
							className="secondary"
							onClick={() => setEvents([])}
							type="button"
						>
							Clear
						</button>
					</div>
				</div>
			</div>
			<div className="event-log">
				{events.length === 0
					? "No events yet."
					: events.map((entry, index) => (
							<div
								className="event-entry"
								key={`${entry}-${index}`}
							>
								{entry}
							</div>
						))}
			</div>
		</div>
	);
}

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
		name: selectedActor as keyof typeof registry,
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

function RawWebSocketPanel({ page }: { page: PageConfig }) {
	const [selectedActor, setSelectedActor] = useState(page.actors[0] ?? "");
	const [message, setMessage] = useState('{"type": "ping"}');
	const [log, setLog] = useState<string[]>([]);
	const socketRef = useRef<WebSocket | null>(null);

	useEffect(() => {
		setSelectedActor(page.actors[0] ?? "");
	}, [page.id, page.actors]);

	const actor = useActor({
		name: selectedActor as keyof typeof registry,
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

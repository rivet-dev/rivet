import { createRivetKit } from "@rivetkit/react";
import { createClient } from "rivetkit/client";
import Editor from "@monaco-editor/react";
import { useEffect, useRef, useState } from "react";
import type {
	ChatMessage,
	CodeAgentState,
	CodeUpdateEvent,
	ResponseEvent,
	registry,
} from "../src/actors/index.ts";

const rivetEndpoint = `${location.origin}/api/rivet`;

const { useActor } = createRivetKit<typeof registry>(rivetEndpoint);

// Raw client for dynamicRunner (actions are unknown at compile time)
const client = createClient<typeof registry>({
	endpoint: rivetEndpoint,
	encoding: "json",
});

const REASONING_OPTIONS = [
	{ value: "none", label: "None" },
	{ value: "medium", label: "Medium" },
	{ value: "high", label: "High" },
	{ value: "extra_high", label: "Extra High" },
] as const;

// Chat column: interacts with the codeAgent actor
function ChatColumn({
	actorKey,
	code,
	onApiKeyStatus,
	onCodeUpdate,
}: {
	actorKey: string;
	code: string;
	onApiKeyStatus: (missing: boolean) => void;
	onCodeUpdate: (code: string, revision: number) => void;
}) {
	const agent = useActor({
		name: "codeAgent",
		key: [actorKey],
	});
	const [messages, setMessages] = useState<ChatMessage[]>([]);
	const [status, setStatus] = useState<string>("idle");
	const [error, setError] = useState<string | null>(null);
	const [input, setInput] = useState("");
	const [reasoning, setReasoning] = useState("none");
	const timelineRef = useRef<HTMLDivElement>(null);

	useEffect(() => {
		if (!agent.connection) return;
		agent.connection.getState().then((state: CodeAgentState) => {
			setMessages(state.messages);
			setStatus(state.status);
			onApiKeyStatus(!state.hasApiKey);
			onCodeUpdate(state.code, state.codeRevision);
		});
	}, [agent.connection]);

	// Scroll to bottom when messages change
	useEffect(() => {
		if (timelineRef.current) {
			timelineRef.current.scrollTop = timelineRef.current.scrollHeight;
		}
	}, [messages]);

	agent.useEvent("response", (payload: ResponseEvent) => {
		if (payload.error) {
			setError(payload.error);
		} else if (payload.done) {
			setError(null);
		}
		setMessages((prev) => {
			const exists = prev.some((msg) => msg.id === payload.messageId);
			if (!exists) {
				// Assistant message placeholder from backend; add it.
				return [
					...prev,
					{
						id: payload.messageId,
						role: "assistant" as const,
						content: payload.content,
						createdAt: Date.now(),
					},
				];
			}
			return prev.map((msg) =>
				msg.id === payload.messageId
					? { ...msg, content: payload.content }
					: msg,
			);
		});
	});

	agent.useEvent("codeUpdated", (payload: CodeUpdateEvent) => {
		onCodeUpdate(payload.code, payload.revision);
	});

	agent.useEvent("statusChanged", (nextStatus: string) => {
		setStatus(nextStatus);
	});

	const sendMessage = async () => {
		if (!agent.connection) return;
		const trimmed = input.trim();
		if (!trimmed) return;

		setError(null);
		// Optimistic add
		const userMsg: ChatMessage = {
			id: `pending-${Date.now()}`,
			role: "user",
			content: trimmed,
			createdAt: Date.now(),
		};
		setMessages((prev) => [...prev, userMsg]);
		setInput("");

		// Send the current editor code along with the message so the AI can
		// modify existing code rather than generating from scratch.
		await agent.connection.send("chat", {
			text: trimmed,
			currentCode: code,
			reasoning,
		});
	};

	const handleKeyDown = (event: React.KeyboardEvent<HTMLTextAreaElement>) => {
		if (event.key === "Enter" && !event.shiftKey) {
			event.preventDefault();
			sendMessage();
		}
	};

	return (
		<div className="column">
			<div className="column-header">
				<span className="column-title">Chat</span>
				<div className="chat-header__right">
					<span className="model-label">GPT-4o</span>
					<select
						value={reasoning}
						onChange={(e) => setReasoning(e.target.value)}
						className="model-select"
						title="Reasoning level"
					>
						{REASONING_OPTIONS.map((opt) => (
							<option key={opt.value} value={opt.value}>
								{opt.label}
							</option>
						))}
					</select>
					<span
						className={`status-dot status-dot--${status}`}
						title={status}
					/>
				</div>
			</div>
			{error && (
				<div className="error-banner" onClick={() => setError(null)}>
					{error}
				</div>
			)}
			<div className="column-body" ref={timelineRef}>
				{messages.length === 0 ? (
					<p className="empty-state">
						Describe the actor you want to build.
					</p>
				) : (
					messages.map((msg) => (
						<div
							key={msg.id}
							className={`chat-message chat-message--${msg.role}`}
						>
							<div className="chat-message__role">
								{msg.role === "user" ? "You" : "AI"}
							</div>
							<div className="chat-message__content">
								{msg.content || (
									<span className="thinking-indicator">
										Thinking...
									</span>
								)}
							</div>
						</div>
					))
				)}
			</div>
			<div className="column-footer">
				<textarea
					value={input}
					onChange={(e) => setInput(e.target.value)}
					onKeyDown={handleKeyDown}
					placeholder="Describe your actor..."
					rows={2}
					disabled={!agent.connection}
				/>
				<button
					onClick={sendMessage}
					disabled={!agent.connection || !input.trim()}
				>
					Send
				</button>
			</div>
		</div>
	);
}

// Code column: Monaco editor for viewing and editing generated code
function CodeColumn({
	code,
	onCodeChange,
}: {
	code: string;
	onCodeChange: (code: string) => void;
}) {
	return (
		<div className="column">
			<div className="column-header">
				<span className="column-title">Code Editor</span>
				</div>
			<div className="column-body code-body">
				<Editor
					height="100%"
					defaultLanguage="javascript"
					theme="vs-dark"
					value={code}
					onChange={(value) => onCodeChange(value || "")}
					options={{
						minimap: { enabled: false },
						fontSize: 13,
						lineNumbers: "on",
						tabSize: 2,
						scrollBeyondLastLine: false,
						wordWrap: "on",
						padding: { top: 12 },
					}}
				/>
			</div>
		</div>
	);
}

// Action log entry
type ActionLogEntry = {
	id: string;
	action: string;
	args: string;
	result?: string;
	error?: string;
	timestamp: number;
};

// Actor interface column: generic interface to interact with the dynamic actor
function ActorInterfaceColumn({
	actorKey,
	deployVersion,
	onDeploy,
}: {
	actorKey: string;
	deployVersion: number;
	onDeploy: () => void;
}) {
	const [actionName, setActionName] = useState("increment");
	const [actionArgs, setActionArgs] = useState("[1]");
	const [log, setLog] = useState<ActionLogEntry[]>([]);
	const [calling, setCalling] = useState(false);
	const logRef = useRef<HTMLDivElement>(null);

	useEffect(() => {
		if (logRef.current) {
			logRef.current.scrollTop = logRef.current.scrollHeight;
		}
	}, [log]);

	// Reset the action log when the actor key or deploy version changes.
	useEffect(() => {
		setLog([]);
	}, [actorKey, deployVersion]);

	const callAction = async () => {
		if (!actionName.trim()) return;

		let parsedArgs: unknown[];
		try {
			parsedArgs = JSON.parse(actionArgs);
			if (!Array.isArray(parsedArgs)) {
				parsedArgs = [parsedArgs];
			}
		} catch {
			setLog((prev) => [
				...prev,
				{
					id: String(Date.now()),
					action: actionName,
					args: actionArgs,
					error: "Invalid JSON for args. Must be a JSON array, e.g. [5]",
					timestamp: Date.now(),
				},
			]);
			return;
		}

		setCalling(true);

		const entry: ActionLogEntry = {
			id: String(Date.now()),
			action: actionName,
			args: JSON.stringify(parsedArgs),
			timestamp: Date.now(),
		};

		try {
			const handle = client.dynamicRunner.getOrCreate([
				actorKey,
				String(deployVersion),
			]);
			const result = await (handle as any)[actionName](...parsedArgs);
			entry.result = JSON.stringify(result, null, 2);
		} catch (error) {
			entry.error =
				error instanceof Error ? error.message : "Action call failed";
		}

		setLog((prev) => [...prev, entry]);
		setCalling(false);
	};

	const handleKeyDown = (event: React.KeyboardEvent<HTMLInputElement>) => {
		if (event.key === "Enter") {
			callAction();
		}
	};

	return (
		<div className="column">
			<div className="column-header">
				<span className="column-title">Actor Interface</span>
				<button className="deploy-button" onClick={onDeploy}>
					Deploy
				</button>
			</div>
			<div className="column-body">
				<div className="interface-section">
					<label className="interface-label">Action</label>
					<input
						type="text"
						value={actionName}
						onChange={(e) => setActionName(e.target.value)}
						onKeyDown={handleKeyDown}
						placeholder="actionName"
						className="interface-input"
					/>
					<label className="interface-label">Args (JSON array)</label>
					<input
						type="text"
						value={actionArgs}
						onChange={(e) => setActionArgs(e.target.value)}
						onKeyDown={handleKeyDown}
						placeholder='[1, "hello"]'
						className="interface-input"
					/>
					<button
						onClick={callAction}
						disabled={calling || !actionName.trim()}
						className="call-button"
					>
						{calling ? "Calling..." : "Call Action"}
					</button>
				</div>

				<div className="interface-section">
					<label className="interface-label">Log</label>
					<div className="action-log" ref={logRef}>
						{log.length === 0 ? (
							<p className="empty-state">
								Deploy the actor and call an action to see
								results.
							</p>
						) : (
							log.map((entry) => (
								<div key={entry.id} className="log-entry">
									<div className="log-entry__header">
										<span className="log-entry__action">
											{entry.action}(
											{entry.args})
										</span>
										<span className="log-entry__time">
											{new Date(
												entry.timestamp,
											).toLocaleTimeString()}
										</span>
									</div>
									{entry.error ? (
										<div className="log-entry__error">
											{entry.error}
										</div>
									) : (
										<div className="log-entry__result">
											{entry.result}
										</div>
									)}
								</div>
							))
						)}
					</div>
				</div>
			</div>
		</div>
	);
}

export function App() {
	const [actorKey, setActorKey] = useState("my-actor");
	const [deployVersion, setDeployVersion] = useState(1);
	const [apiKeyMissing, setApiKeyMissing] = useState(false);
	const [code, setCode] = useState("");
	const [codeRevision, setCodeRevision] = useState(0);

	const handleDeploy = async () => {
		// Save the current editor code to the codeAgent before deploying so the
		// dynamic runner picks up any manual edits.
		const handle = client.codeAgent.getOrCreate([actorKey]);
		await (handle as any).setCode(code);
		setDeployVersion((v) => v + 1);
	};

	const handleCodeUpdate = (newCode: string, revision: number) => {
		setCode(newCode);
		setCodeRevision(revision);
	};

	useEffect(() => {
		setDeployVersion(1);
	}, [actorKey]);

	return (
		<div className="app">
			{apiKeyMissing && (
				<div className="error-banner">
					Missing OPENAI_API_KEY environment variable. Set it and
					restart the server to enable AI code generation.
				</div>
			)}
			<header className="top-bar">
				<span className="top-bar__title">AI-Generated Actor</span>
				<div className="top-bar__key">
					<label>Actor Key</label>
					<input
						type="text"
						value={actorKey}
						onChange={(e) => setActorKey(e.target.value)}
						placeholder="my-actor"
					/>
				</div>
			</header>
			<div className="columns">
				<ChatColumn
					actorKey={actorKey}
					code={code}
					onApiKeyStatus={setApiKeyMissing}
					onCodeUpdate={handleCodeUpdate}
				/>
				<CodeColumn
					code={code}
					onCodeChange={setCode}
				/>
				<ActorInterfaceColumn
					actorKey={actorKey}
					deployVersion={deployVersion}
					onDeploy={handleDeploy}
				/>
			</div>
		</div>
	);
}

import { createRivetKit } from "@rivetkit/react";
import { useEffect, useRef, useState } from "react";
import type {
	ChatMessage,
	CodeAgentState,
	CodeUpdateEvent,
	ResponseEvent,
	registry,
} from "../src/actors.ts";

const { useActor } = createRivetKit<typeof registry>(
	`${location.origin}/api/rivet`,
);

// Chat column: interacts with the codeAgent actor
function ChatColumn({ actorKey }: { actorKey: string }) {
	const agent = useActor({
		name: "codeAgent",
		key: [actorKey],
	});
	const [messages, setMessages] = useState<ChatMessage[]>([]);
	const [status, setStatus] = useState<string>("idle");
	const [input, setInput] = useState("");
	const timelineRef = useRef<HTMLDivElement>(null);

	useEffect(() => {
		if (!agent.connection) return;
		agent.connection.getState().then((state: CodeAgentState) => {
			setMessages(state.messages);
			setStatus(state.status);
		});
	}, [agent.connection]);

	// Scroll to bottom when messages change
	useEffect(() => {
		if (timelineRef.current) {
			timelineRef.current.scrollTop = timelineRef.current.scrollHeight;
		}
	}, [messages]);

	agent.useEvent("response", (payload: ResponseEvent) => {
		setMessages((prev) =>
			prev.map((msg) =>
				msg.id === payload.messageId
					? { ...msg, content: payload.content }
					: msg,
			),
		);
	});

	agent.useEvent("statusChanged", (nextStatus: string) => {
		setStatus(nextStatus);
	});

	const sendMessage = async () => {
		if (!agent.connection) return;
		const trimmed = input.trim();
		if (!trimmed) return;

		// Optimistic add
		const userMsg: ChatMessage = {
			id: `pending-${Date.now()}`,
			role: "user",
			content: trimmed,
			createdAt: Date.now(),
		};
		setMessages((prev) => [...prev, userMsg]);
		setInput("");

		await agent.connection.send("chat", { text: trimmed });
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
				<span
					className={`status-dot status-dot--${status}`}
					title={status}
				/>
			</div>
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

// Code column: shows the current generated code
function CodeColumn({ actorKey }: { actorKey: string }) {
	const agent = useActor({
		name: "codeAgent",
		key: [actorKey],
	});
	const [code, setCode] = useState("");
	const [revision, setRevision] = useState(0);

	useEffect(() => {
		if (!agent.connection) return;
		agent.connection.getState().then((state: CodeAgentState) => {
			setCode(state.code);
			setRevision(state.codeRevision);
		});
	}, [agent.connection]);

	agent.useEvent("codeUpdated", (payload: CodeUpdateEvent) => {
		setCode(payload.code);
		setRevision(payload.revision);
	});

	return (
		<div className="column">
			<div className="column-header">
				<span className="column-title">Generated Code</span>
				<span className="revision-badge">v{revision}</span>
			</div>
			<div className="column-body code-body">
				<pre className="code-block">
					<code>{code}</code>
				</pre>
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
	const [actionName, setActionName] = useState("getCount");
	const [actionArgs, setActionArgs] = useState("[]");
	const [log, setLog] = useState<ActionLogEntry[]>([]);
	const [calling, setCalling] = useState(false);
	const logRef = useRef<HTMLDivElement>(null);

	useEffect(() => {
		if (logRef.current) {
			logRef.current.scrollTop = logRef.current.scrollHeight;
		}
	}, [log]);

	// Reset log when deploy version changes
	useEffect(() => {
		setLog([]);
	}, [deployVersion]);

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
			const response = await fetch(
				`/api/dynamic/${encodeURIComponent(actorKey)}/${deployVersion}/action`,
				{
					method: "POST",
					headers: { "content-type": "application/json" },
					body: JSON.stringify({ name: actionName, args: parsedArgs }),
				},
			);
			const data = await response.json();
			if (data.error) {
				entry.error = data.error;
			} else {
				entry.result = JSON.stringify(data.result, null, 2);
			}
		} catch (error) {
			entry.error =
				error instanceof Error ? error.message : "Request failed";
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
					Deploy v{deployVersion}
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

	const handleDeploy = () => {
		setDeployVersion((v) => v + 1);
	};

	return (
		<div className="app">
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
				<ChatColumn actorKey={actorKey} />
				<CodeColumn actorKey={actorKey} />
				<ActorInterfaceColumn
					actorKey={actorKey}
					deployVersion={deployVersion}
					onDeploy={handleDeploy}
				/>
			</div>
		</div>
	);
}

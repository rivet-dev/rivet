import { createRivetKit } from "@rivetkit/react";
import { useEffect, useState } from "react";
import type {
	AgentInfo,
	AgentMessage,
	AgentStatus,
	registry,
} from "../src/actors.ts";

const { useActor } = createRivetKit<typeof registry>(
	`${location.origin}/api/rivet`,
);

type ResponseEvent = {
	messageId: string;
	delta: string;
	content: string;
	done: boolean;
	error?: string;
};

function formatTime(timestamp: number) {
	return new Date(timestamp).toLocaleTimeString();
}

function AgentPanel({ info }: { info: AgentInfo }) {
	const agent = useActor({
		name: "agent",
		key: [info.id],
	});
	const [messages, setMessages] = useState<AgentMessage[]>([]);
	const [status, setStatus] = useState<AgentStatus | null>(null);
	const [input, setInput] = useState("");

	useEffect(() => {
		if (!agent.connection) {
			return;
		}

		agent.connection.getHistory().then(setMessages);
		agent.connection.getStatus().then(setStatus);
	}, [agent.connection]);

	agent.useEvent("messageAdded", (message: AgentMessage) => {
		setMessages((prev) => {
			const existingIndex = prev.findIndex((item) => item.id === message.id);
			if (existingIndex !== -1) {
				const next = [...prev];
				next[existingIndex] = message;
				return next;
			}
			return [...prev, message].sort(
				(a, b) => a.createdAt - b.createdAt,
			);
		});
	});

	agent.useEvent("response", (payload: ResponseEvent) => {
		setMessages((prev) =>
			prev.map((message) =>
				message.id === payload.messageId
					? { ...message, content: payload.content }
					: message,
			),
		);
	});

	agent.useEvent("status", (nextStatus: AgentStatus) => {
		setStatus(nextStatus);
	});

	const sendMessage = async () => {
		if (!agent.connection) {
			return;
		}

		const trimmed = input.trim();
		if (!trimmed) {
			return;
		}

		await agent.connection.queue.message.send({
			text: trimmed,
			sender: "Operator",
		});
		setInput("");
	};

	const handleKeyDown = (event: React.KeyboardEvent<HTMLTextAreaElement>) => {
		if (event.key === "Enter" && !event.shiftKey) {
			event.preventDefault();
			sendMessage();
		}
	};

	return (
		<section className="agent-card">
			<header className="agent-card__header">
				<div>
					<p className="agent-card__title">{info.name}</p>
					<p className="agent-card__meta">{info.id}</p>
				</div>
				<div className="agent-card__status">
					<span
						className={`status-dot status-dot--${status?.state ?? "idle"}`}
					/>
					<span className="status-label">
						{status?.state ?? "idle"}
					</span>
				</div>
			</header>

			<div className="agent-card__timeline">
				{messages.length === 0 ? (
					<p className="agent-card__empty">
						Send a message to wake this Rivet Actor.
					</p>
				) : (
					messages.map((message) => (
						<article
							key={message.id}
							className={`message message--${message.role}`}
						>
							<div className="message__header">
								<span className="message__sender">{message.sender}</span>
								<span className="message__time">
									{formatTime(message.createdAt)}
								</span>
							</div>
							<p className="message__content">{message.content}</p>
						</article>
					))
				)}
			</div>

			<div className="agent-card__footer">
				<textarea
					value={input}
					onChange={(event) => setInput(event.target.value)}
					onKeyDown={handleKeyDown}
					placeholder="Send a message to this agent"
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

			{status?.state === "error" && status.error ? (
				<p className="agent-card__error">{status.error}</p>
			) : null}
		</section>
	);
}

export function App() {
	const manager = useActor({
		name: "agentManager",
		key: ["primary"],
	});
	const [agents, setAgents] = useState<AgentInfo[]>([]);
	const [agentName, setAgentName] = useState("");

	useEffect(() => {
		if (!manager.connection) {
			return;
		}

		manager.connection.listAgents().then(setAgents);
	}, [manager.connection]);

	const createAgent = async () => {
		if (!manager.connection) {
			return;
		}

		const info = await manager.connection.createAgent(agentName);
		setAgents((prev) => [...prev, info]);
		setAgentName("");
	};

	const handleCreateKeyDown = (
		event: React.KeyboardEvent<HTMLInputElement>,
	) => {
		if (event.key === "Enter") {
			createAgent();
		}
	};

	return (
		<div className="layout">
			<header className="hero">
				<div>
					<p className="hero__eyebrow">Sandbox Coding Agent</p>
					<h1>Run coding agents in isolated sandboxes.</h1>
					<p className="hero__subtitle">
						Each Rivet Actor streams sandboxed agent output as queue
						messages arrive.
					</p>
				</div>
				<div className="hero__controls">
					<label className="control">
						<span>Agent name</span>
						<input
							value={agentName}
							onChange={(event) => setAgentName(event.target.value)}
							onKeyDown={handleCreateKeyDown}
							placeholder="Ops Analyst"
							disabled={!manager.connection}
						/>
					</label>
					<button
						onClick={createAgent}
						disabled={!manager.connection}
					>
						Create agent
					</button>
				</div>
			</header>

			<section className="agents">
				<div className="agents__header">
					<h2>Active agents</h2>
					<span className="agents__count">{agents.length}</span>
				</div>
				<div className="agents__grid">
					{agents.length === 0 ? (
						<p className="agents__empty">
							No agents yet. Create one to start coding.
						</p>
					) : (
						agents.map((info) => (
							<AgentPanel key={info.id} info={info} />
						))
					)}
				</div>
			</section>
		</div>
	);
}

import { createRivetKit } from "@rivetkit/react";
import { useEffect, useState, useRef, useCallback } from "react";
import { registry } from "../src/registry";
import { type PromptMessage, type ResponseChunk } from "../src/shared/types";
import { getStreams, getStreamPaths } from "../src/shared/streams";
import "./App.css";

const { useActor } = createRivetKit<typeof registry>(`${window.location.origin}/api/rivet`);

interface Message {
	id: string;
	role: "user" | "assistant";
	content: string;
	isStreaming?: boolean;
}

export function App() {
	const [conversationId, setConversationId] = useState("my-chat");
	const [conversationInput, setConversationInput] = useState("my-chat");
	const [messages, setMessages] = useState<Message[]>([]);
	const [input, setInput] = useState("");
	const [isLoading, setIsLoading] = useState(false);
	const [rawPrompts, setRawPrompts] = useState<string[]>([]);
	const [rawResponses, setRawResponses] = useState<string[]>([]);
	const [promptArrowActive, setPromptArrowActive] = useState(false);
	const [responseArrowActive, setResponseArrowActive] = useState(false);
	const responseStreamRef = useRef<AbortController | null>(null);
	const messagesEndRef = useRef<HTMLDivElement>(null);
	const promptsEndRef = useRef<HTMLDivElement>(null);
	const responsesEndRef = useRef<HTMLDivElement>(null);
	const inputRef = useRef<HTMLTextAreaElement>(null);

	const handleConversationChange = () => {
		if (conversationInput.trim() && conversationInput !== conversationId) {
			// Abort existing response stream
			responseStreamRef.current?.abort();
			// Reset state for new conversation
			setConversationId(conversationInput.trim());
			setMessages([]);
			setRawPrompts([]);
			setRawResponses([]);
			setIsLoading(false);
		}
	};

	// Connect to the AI agent actor for this conversation
	const _aiAgent = useActor({
		name: "aiAgent",
		key: [conversationId],
		createWithInput: { conversationId },
		enabled: true,
	});

	// Auto-scroll to bottom when messages change
	useEffect(() => {
		messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
	}, [messages]);

	useEffect(() => {
		promptsEndRef.current?.scrollIntoView({ behavior: "smooth" });
	}, [rawPrompts]);

	useEffect(() => {
		responsesEndRef.current?.scrollIntoView({ behavior: "smooth" });
	}, [rawResponses]);

	// Set up stream listeners and load history
	useEffect(() => {
		const abortController = new AbortController();
		responseStreamRef.current = abortController;

		// Store prompts and responses with timestamps for proper ordering
		const promptsMap = new Map<string, PromptMessage>();
		const responsesMap = new Map<string, { content: string; isComplete: boolean; timestamp: number }>();

		const rebuildMessages = () => {
			const newMessages: Message[] = [];

			// Sort prompts by timestamp
			const sortedPrompts = Array.from(promptsMap.values()).sort((a, b) => a.timestamp - b.timestamp);

			for (const prompt of sortedPrompts) {
				// Add user message
				newMessages.push({
					id: `prompt-${prompt.id}`,
					role: "user",
					content: prompt.content,
				});

				// Add corresponding response if exists
				const response = responsesMap.get(prompt.id);
				if (response && response.content) {
					newMessages.push({
						id: `response-${prompt.id}`,
						role: "assistant",
						content: response.content,
						isStreaming: !response.isComplete,
					});
				}
			}

			setMessages(newMessages);
		};

		const loadStreams = async () => {
			const { promptStream, responseStream } = await getStreams(conversationId);

			// Track if we're still loading history (don't flash arrows during history load)
			let isLoadingHistory = true;

			// Load prompt history and continue listening
			const consumePrompts = async () => {
				try {
					for await (const data of promptStream.json<PromptMessage | PromptMessage[]>({ live: "long-poll" })) {
						if (abortController.signal.aborted) break;

						const prompts = Array.isArray(data) ? data : [data];

						for (const prompt of prompts) {
							if (!prompt.id) continue;

							// Add raw prompt to debug panel
							setRawPrompts((prev) => {
								if (prev.some(p => p.includes(prompt.id))) return prev;
								return [...prev, JSON.stringify(prompt)];
							});

							// Store prompt and rebuild messages
							if (!promptsMap.has(prompt.id)) {
								promptsMap.set(prompt.id, prompt);
								rebuildMessages();
							}

							if (!isLoadingHistory) {
								setPromptArrowActive(true);
								setTimeout(() => setPromptArrowActive(false), 300);
							}
						}
					}
				} catch (error) {
					if (!abortController.signal.aborted) {
						console.error("Error consuming prompts:", error);
					}
				}
			};

			// Load response history and continue listening
			const consumeResponses = async () => {
				try {
					for await (const data of responseStream.json<ResponseChunk | ResponseChunk[]>({ live: "long-poll" })) {
						if (abortController.signal.aborted) break;

						const responses = Array.isArray(data) ? data : [data];

						for (const response of responses) {
							if (!response.promptId) continue;

							// Add raw response to debug panel
							setRawResponses((prev) => [...prev, JSON.stringify(response)]);

							// Update response map
							const existing = responsesMap.get(response.promptId);
							if (response.isComplete) {
								if (existing) {
									existing.isComplete = true;
								} else {
									responsesMap.set(response.promptId, {
										content: response.content,
										isComplete: true,
										timestamp: response.timestamp,
									});
								}
							} else {
								if (existing) {
									existing.content += response.content;
								} else {
									responsesMap.set(response.promptId, {
										content: response.content,
										isComplete: false,
										timestamp: response.timestamp,
									});
								}
							}

							rebuildMessages();

							if (!isLoadingHistory) {
								setResponseArrowActive(true);
								setTimeout(() => setResponseArrowActive(false), 300);
							}

							if (response.isComplete) {
								setIsLoading(false);
							}
						}
					}
				} catch (error) {
					if (!abortController.signal.aborted) {
						console.error("Error consuming responses:", error);
					}
				}
			};

			// Start consuming both streams
			consumePrompts();
			consumeResponses();

			// After a short delay, consider history loaded
			setTimeout(() => {
				isLoadingHistory = false;
			}, 500);
		};

		loadStreams();

		return () => {
			abortController.abort();
		};
	}, [conversationId]);

	const handleSendMessage = useCallback(async () => {
		if (!input.trim() || isLoading) return;

		const promptId = crypto.randomUUID();
		const prompt: PromptMessage = {
			id: promptId,
			content: input.trim(),
			timestamp: Date.now(),
		};

		setInput("");
		setIsLoading(true);

		const { promptStream } = await getStreams(conversationId);

		// Write to stream - the stream listener will pick it up and update UI
		await promptStream.append(JSON.stringify(prompt) + "\n", { contentType: "application/json" });
	}, [input, isLoading, conversationId]);

	const handleKeyDown = (e: React.KeyboardEvent) => {
		if (e.key === "Enter" && !e.shiftKey) {
			e.preventDefault();
			handleSendMessage();
		}
	};

	const { promptStreamPath, responseStreamPath } = getStreamPaths(conversationId);

	return (
		<div className="app">
				<header className="header">
					<div className="header-left">
						<div className="logo">DS</div>
						<h1>Durable Streams AI Agent</h1>
					</div>
				</header>

				<div className="main-layout">
					<div className="chat-panel">
						<div className="panel-header">
							<span className="panel-title">Chat:</span>
							<input
								type="text"
								className="conversation-input"
								value={conversationInput}
								onChange={(e) => setConversationInput(e.target.value)}
								onKeyDown={(e) => e.key === "Enter" && handleConversationChange()}
								onBlur={handleConversationChange}
								placeholder="chat-name"
							/>
						</div>
						<div className="messages-container">
							{messages.length === 0 ? (
								<div className="empty-state">
									<p>Send a message to see how it flows through durable streams to the AI agent.</p>
								</div>
							) : (
								<div className="messages">
									{messages.map((msg) => (
										<div key={msg.id} className={`message ${msg.role}`}>
											<div className="avatar">
												{msg.role === "user" ? "U" : "AI"}
											</div>
											<div className="message-content">
												{msg.content}
												{msg.isStreaming && <span className="cursor" />}
											</div>
										</div>
									))}
									<div ref={messagesEndRef} />
								</div>
							)}
						</div>
						<div className="input-container">
							<div className="input-wrapper">
								<textarea
									ref={inputRef}
									value={input}
									onChange={(e) => setInput(e.target.value)}
									onKeyDown={handleKeyDown}
									placeholder="Send a message..."
									disabled={isLoading}
									rows={1}
								/>
								<button
									className="send-button"
									onClick={handleSendMessage}
									disabled={isLoading || !input.trim()}
								>
									<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
										<line x1="22" y1="2" x2="11" y2="13" />
										<polygon points="22 2 15 22 11 13 2 9 22 2" />
									</svg>
								</button>
							</div>
						</div>
					</div>

					<div className="right-panel">
						<div className="diagram-container">
							<svg width="100%" height="280" viewBox="0 0 400 280">
								{/* Define arrow markers */}
								<defs>
									<marker id="arrowhead" markerWidth="10" markerHeight="7" refX="9" refY="3.5" orient="auto">
										<polygon points="0 0, 10 3.5, 0 7" className={`arrow-head`} />
									</marker>
									<marker id="arrowhead-active" markerWidth="10" markerHeight="7" refX="9" refY="3.5" orient="auto">
										<polygon points="0 0, 10 3.5, 0 7" className={`arrow-head active`} />
									</marker>
								</defs>

								{/* Prompts Stream - top center */}
								<rect x="150" y="20" width="100" height="50" rx="12" className={`diagram-node-rect stream ${promptArrowActive ? 'active' : ''}`} />
								<text x="200" y="40" textAnchor="middle" className="diagram-node-text">Prompts</text>
								<text x="200" y="55" textAnchor="middle" className="diagram-node-text">Stream</text>
								<text x="200" y="80" textAnchor="middle" className={`diagram-node-count ${promptArrowActive ? 'active' : ''}`}>{rawPrompts.length} entries</text>

								{/* Browser - left middle */}
								<rect x="20" y="115" width="90" height="50" rx="12" className="diagram-node-rect" />
								<text x="65" y="145" textAnchor="middle" className="diagram-node-text">Browser</text>

								{/* Agent Actor - right middle */}
								<rect x="290" y="115" width="90" height="50" rx="12" className="diagram-node-rect" />
								<text x="335" y="135" textAnchor="middle" className="diagram-node-text">Agent</text>
								<text x="335" y="150" textAnchor="middle" className="diagram-node-text">Actor</text>

								{/* Response Stream - bottom center */}
								<rect x="150" y="210" width="100" height="50" rx="12" className={`diagram-node-rect stream ${responseArrowActive ? 'active' : ''}`} />
								<text x="200" y="230" textAnchor="middle" className="diagram-node-text">Response</text>
								<text x="200" y="245" textAnchor="middle" className="diagram-node-text">Stream</text>
								<text x="200" y="200" textAnchor="middle" className={`diagram-node-count ${responseArrowActive ? 'active' : ''}`}>{rawResponses.length} entries</text>

								{/* Browser -> Prompts Stream */}
								<path
									d="M 95 115 Q 120 80 150 55"
									className={`arrow ${promptArrowActive ? 'active' : ''}`}
									markerEnd={promptArrowActive ? "url(#arrowhead-active)" : "url(#arrowhead)"}
								/>
								<text x="100" y="75" fontSize="10" fill={promptArrowActive ? '#ff4f00' : '#6e6e73'}>Prompt</text>

								{/* Prompts Stream -> Agent */}
								<path
									d="M 250 55 Q 280 80 305 115"
									className={`arrow ${promptArrowActive ? 'active' : ''}`}
									markerEnd={promptArrowActive ? "url(#arrowhead-active)" : "url(#arrowhead)"}
								/>
								<text x="270" y="75" fontSize="10" fill={promptArrowActive ? '#ff4f00' : '#6e6e73'}>Prompt</text>

								{/* Agent -> Response Stream */}
								<path
									d="M 305 165 Q 280 200 250 225"
									className={`arrow ${responseArrowActive ? 'active' : ''}`}
									markerEnd={responseArrowActive ? "url(#arrowhead-active)" : "url(#arrowhead)"}
								/>
								<text x="270" y="205" fontSize="10" fill={responseArrowActive ? '#ff4f00' : '#6e6e73'}>Tokens</text>

								{/* Response Stream -> Browser */}
								<path
									d="M 150 225 Q 120 200 95 165"
									className={`arrow ${responseArrowActive ? 'active' : ''}`}
									markerEnd={responseArrowActive ? "url(#arrowhead-active)" : "url(#arrowhead)"}
								/>
								<text x="100" y="205" fontSize="10" fill={responseArrowActive ? '#ff4f00' : '#6e6e73'}>Tokens</text>
							</svg>
						</div>

						<div className="streams-panel">
							<div className="stream-section">
								<div className="stream-header">
									<div className="stream-header-left">
										<span className="stream-title">Prompts</span>
										<span className="stream-badge">{rawPrompts.length}</span>
									</div>
									<a
										className="stream-link"
										href={`http://localhost:3000/stream/${encodeURIComponent(promptStreamPath)}`}
										target="_blank"
										rel="noopener noreferrer"
									>
										Open
										<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
											<path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6" />
											<polyline points="15 3 21 3 21 9" />
											<line x1="10" y1="14" x2="21" y2="3" />
										</svg>
									</a>
								</div>
								<div className="stream-content">
									{rawPrompts.length === 0 ? (
										<div className="stream-empty">No prompts yet</div>
									) : (
										<>
											{rawPrompts.map((entry, i) => (
												<div key={i} className="stream-entry">
													{formatJson(entry)}
												</div>
											))}
											<div ref={promptsEndRef} />
										</>
									)}
								</div>
							</div>

							<div className="stream-section">
								<div className="stream-header">
									<div className="stream-header-left">
										<span className="stream-title">Responses</span>
										<span className="stream-badge">{rawResponses.length}</span>
									</div>
									<a
										className="stream-link"
										href={`http://localhost:3000/stream/${encodeURIComponent(responseStreamPath)}`}
										target="_blank"
										rel="noopener noreferrer"
									>
										Open
										<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
											<path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6" />
											<polyline points="15 3 21 3 21 9" />
											<line x1="10" y1="14" x2="21" y2="3" />
										</svg>
									</a>
								</div>
								<div className="stream-content">
									{rawResponses.length === 0 ? (
										<div className="stream-empty">No responses yet</div>
									) : (
										<>
											{rawResponses.map((entry, i) => (
												<div key={i} className="stream-entry">
													{formatJson(entry)}
												</div>
											))}
											<div ref={responsesEndRef} />
										</>
									)}
								</div>
							</div>
						</div>
					</div>
				</div>
			</div>
	);
}

function formatJson(jsonStr: string): React.ReactNode {
	try {
		const obj = JSON.parse(jsonStr);
		return formatValue(obj);
	} catch {
		return jsonStr;
	}
}

function formatValue(value: unknown, depth = 0): React.ReactNode {
	if (value === null) return <span className="boolean">null</span>;
	if (typeof value === "boolean") return <span className="boolean">{String(value)}</span>;
	if (typeof value === "number") return <span className="number">{value}</span>;
	if (typeof value === "string") return <span className="string">"{value}"</span>;

	if (Array.isArray(value)) {
		if (value.length === 0) return "[]";
		return (
			<>
				{"[\n"}
				{value.map((item, i) => (
					<span key={i}>
						{"  ".repeat(depth + 1)}
						{formatValue(item, depth + 1)}
						{i < value.length - 1 ? ",\n" : "\n"}
					</span>
				))}
				{"  ".repeat(depth)}]
			</>
		);
	}

	if (typeof value === "object") {
		const entries = Object.entries(value);
		if (entries.length === 0) return "{}";
		return (
			<>
				{"{\n"}
				{entries.map(([key, val], i) => (
					<span key={key}>
						{"  ".repeat(depth + 1)}
						<span className="key">"{key}"</span>: {formatValue(val, depth + 1)}
						{i < entries.length - 1 ? ",\n" : "\n"}
					</span>
				))}
				{"  ".repeat(depth)}{"}"}
			</>
		);
	}

	return String(value);
}

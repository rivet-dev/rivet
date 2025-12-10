import { useEffect, useState, useRef } from "react";
import { useParams, Link, useLocation } from "react-router-dom";
import { client } from "@/lib/client";
import type { AppInfo, UIMessage } from "../../shared/types";
import { MessageCircle, Monitor, Send, Square } from "lucide-react";
import { Button } from "@/components/ui/button";
import ReactMarkdown from "react-markdown";
import { TopBar } from "@/components/TopBar";
import WebView from "@/components/WebView";

export default function AppEditorPage() {
	const { id } = useParams<{ id: string }>();
	const location = useLocation();
	const appId = id!;

	// Check for pending message from navigation state (when creating new app with initial prompt)
	const pendingMessage = (location.state as { pendingMessage?: UIMessage; gitRepo?: string } | null)?.pendingMessage;
	const pendingGitRepo = (location.state as { pendingMessage?: UIMessage; gitRepo?: string } | null)?.gitRepo;
	const hasPendingMessageRef = useRef(false);

	const [appInfo, setAppInfo] = useState<AppInfo | null>(null);
	const [messages, setMessages] = useState<UIMessage[]>([]);
	const [isLoading, setIsLoading] = useState(true);
	const [error, setError] = useState<string | null>(null);
	const [input, setInput] = useState("");
	const [isGenerating, setIsGenerating] = useState(false);
	const [mobileActiveTab, setMobileActiveTab] = useState<"chat" | "preview">("chat");
	const [isMobile, setIsMobile] = useState(false);
	const [devServerUrls, setDevServerUrls] = useState<{
		ephemeralUrl?: string;
		consoleUrl?: string;
		codeServerUrl?: string;
	}>({});
	const [isConnectionReady, setIsConnectionReady] = useState(false);
	const userAppConnectionRef = useRef<any>(null);
	const messagesEndRef = useRef<HTMLDivElement>(null);

	useEffect(() => {
		const checkMobile = () => setIsMobile(window.innerWidth < 768);
		checkMobile();
		window.addEventListener("resize", checkMobile);
		return () => window.removeEventListener("resize", checkMobile);
	}, []);

	useEffect(() => {
		document.body.style.overflow = "hidden";
		return () => { document.body.style.overflow = "auto"; };
	}, []);

	useEffect(() => { loadAppData(); }, [appId]);

	useEffect(() => {
		if (!appInfo) return;
		let mounted = true;

		const setupConnections = async () => {
			try {
				const userAppConnection = await client.userApp.get([appId]).connect();
				if (!mounted) {
					userAppConnection?.dispose?.();
					return;
				}
				userAppConnectionRef.current = userAppConnection;
				userAppConnection.on("newMessage", (message: UIMessage) => {
					if (mounted) {
						// Update existing message if it exists (from streaming), otherwise add new
						setMessages((prev) => {
							const existingIdx = prev.findIndex((m) => m.id === message.id);
							if (existingIdx >= 0) {
								// Replace with final message content
								const updated = [...prev];
								updated[existingIdx] = message;
								return updated;
							}
							return [...prev, message];
						});
					}
				});

				userAppConnection.on("abort", () => {
					if (mounted) setIsGenerating(false);
				});

				// Listen for step updates to show streaming progress
				userAppConnection.on("stepUpdate", ({ id, text }: { id: string; text: string }) => {
					if (mounted && text) {
						setMessages((prev) => {
							// Find if we already have this message (streaming update)
							const existingIdx = prev.findIndex((m) => m.id === id);
							if (existingIdx >= 0) {
								// Update existing streaming message
								const updated = [...prev];
								updated[existingIdx] = {
									...updated[existingIdx],
									parts: [{ type: "text", text }],
								};
								return updated;
							}
							// Add new streaming message
							return [...prev, {
								id,
								role: "assistant" as const,
								parts: [{ type: "text" as const, text }],
							}];
						});
					}
				});

				const status = await client.userApp.get([appId]).getStreamStatus();
				if (mounted) {
					setIsGenerating(status === "running");
					setIsConnectionReady(true);
				}
			} catch (err) {
				console.error("Failed to setup actor connections:", err);
			}
		};

		setupConnections();
		return () => {
			mounted = false;
			setIsConnectionReady(false);
			userAppConnectionRef.current?.dispose?.();
		};
	}, [appId, appInfo]);

	useEffect(() => {
		messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
	}, [messages]);

	// Fetch dev server URLs when app info is loaded
	useEffect(() => {
		if (!appInfo?.gitRepo) return;

		const fetchDevServerUrls = async () => {
			try {
				const result = await client.userApp.get([appId]).requestDevServer();
				setDevServerUrls({
					ephemeralUrl: result.ephemeralUrl,
					consoleUrl: result.consoleUrl,
					codeServerUrl: result.codeServerUrl,
				});
			} catch (err) {
				console.error("Failed to fetch dev server URLs:", err);
			}
		};

		fetchDevServerUrls();
	}, [appId, appInfo?.gitRepo]);

	// Process pending message from navigation state (for new app creation with initial prompt)
	useEffect(() => {
		if (!pendingMessage || !pendingGitRepo || hasPendingMessageRef.current || isLoading || !isConnectionReady) return;

		// Check if this message was already processed (exists in messages loaded from actor)
		// This prevents re-sending on page reload since messages are persisted in actor state
		if (messages.some((m) => m.id === pendingMessage.id)) {
			console.log("[processPendingMessage] Message already exists in actor state, skipping");
			hasPendingMessageRef.current = true;
			// Clear the navigation state to prevent future re-processing
			window.history.replaceState({}, document.title);
			return;
		}

		hasPendingMessageRef.current = true;

		const processPendingMessage = async () => {
			console.log("[processPendingMessage] Processing pending message from navigation state");
			// Optimistically show the user message (actor will broadcast it too, but we show immediately)
			setMessages((prev) => {
				if (prev.some((m) => m.id === pendingMessage.id)) {
					return prev;
				}
				return [...prev, pendingMessage];
			});
			setIsGenerating(true);

			try {
				// sendChatMessage handles everything: adds user message, sets stream status,
				// calls AI, adds assistant message, clears stream, and broadcasts events
				console.log("[processPendingMessage] Calling userApp.sendChatMessage...");
				await client.userApp.get([appId]).sendChatMessage({ message: pendingMessage });
				console.log("[processPendingMessage] Complete!");
				// Clear the navigation state after successful processing
				window.history.replaceState({}, document.title);
			} catch (err) {
				console.error("[processPendingMessage] Error:", err);
			} finally {
				setIsGenerating(false);
			}
		};

		processPendingMessage();
	}, [pendingMessage, pendingGitRepo, isLoading, isConnectionReady, appId, messages]);

	// Helper to deduplicate messages by ID (keeps first occurrence)
	const dedupeMessages = (msgs: UIMessage[]): UIMessage[] => {
		const seen = new Set<string>();
		return msgs.filter((m) => {
			if (seen.has(m.id)) return false;
			seen.add(m.id);
			return true;
		});
	};

	async function loadAppData() {
		try {
			// Use get() for existing actors - the actor should exist if we're on this page
			const userAppHandle = client.userApp.get([appId]);
			const data = await userAppHandle.getAll();
			if (!data.info) {
				setError("App not found");
				setIsLoading(false);
				return;
			}
			setAppInfo(data.info);
			// Deduplicate messages in case actor state has duplicates from before the fix
			setMessages(dedupeMessages(data.messages));

			// Get stream status from userApp
			const status = await userAppHandle.getStreamStatus();
			setIsGenerating(status === "running");
			setIsLoading(false);
		} catch (err) {
			console.error("Failed to load app:", err);
			setError(err instanceof Error ? err.message : "Failed to load app");
			setIsLoading(false);
		}
	}

	const handleSendMessage = async () => {
		console.log("[handleSendMessage] Starting...", { input: input.trim(), isGenerating, gitRepo: appInfo?.gitRepo });
		if (!input.trim() || isGenerating || !appInfo?.gitRepo) {
			console.log("[handleSendMessage] Early return - conditions not met");
			return;
		}

		const userMessage: UIMessage = {
			id: crypto.randomUUID(),
			role: "user",
			parts: [{ type: "text", text: input }],
		};
		console.log("[handleSendMessage] Created user message:", userMessage.id);

		// Optimistically show the user message (actor will broadcast it too, but we show immediately)
		setMessages((prev) => [...prev, userMessage]);
		setInput("");
		setIsGenerating(true);

		try {
			// sendChatMessage handles everything: adds user message, sets stream status,
			// calls AI, adds assistant message, clears stream, and broadcasts events
			console.log("[handleSendMessage] Calling userApp.sendChatMessage...");
			await client.userApp.get([appId]).sendChatMessage({ message: userMessage });
			console.log("[handleSendMessage] Complete!");
		} catch (err) {
			console.error("[handleSendMessage] Error:", err);
		} finally {
			setIsGenerating(false);
			console.log("[handleSendMessage] Finally block - isGenerating set to false");
		}
	};

	const handleStop = async () => {
		await client.userApp.get([appId]).abortStream();
	};

	if (isLoading) {
		return <div className="flex items-center justify-center min-h-screen"><div className="animate-pulse text-lg">Loading app...</div></div>;
	}

	if (error || !appInfo) {
		return (
			<div className="text-center my-16">
				Project not found.
				<div className="flex justify-center mt-4"><Link to="/"><Button>Go back to home</Button></Link></div>
			</div>
		);
	}

	return (
		<div className="h-screen flex flex-col" style={{ height: "100dvh" }}>
			<div className="flex-1 overflow-hidden flex flex-col md:grid md:grid-cols-[1fr_2fr]">
				{/* Chat Panel */}
				<div className={isMobile ? `absolute inset-0 z-10 flex flex-col transition-transform duration-200 bg-background ${mobileActiveTab === "chat" ? "translate-x-0" : "-translate-x-full"}` : "h-full overflow-hidden flex flex-col border-r"} style={isMobile ? { top: "0", bottom: "calc(60px + env(safe-area-inset-bottom))" } : undefined}>
					{/* Top Bar */}
					<TopBar appName={appInfo.name} />

					{/* Messages */}
					<div className="flex-1 overflow-y-auto p-4 space-y-4">
						{messages.map((message) => (
							<div key={message.id} className={`${message.role === "user" ? "flex justify-end" : ""}`}>
								<div className={`max-w-[85%] ${message.role === "user" ? "bg-primary text-primary-foreground rounded-xl px-4 py-2" : ""}`}>
									{message.parts.map((part, idx) =>
										part.type === "text" ? (
											message.role === "user" ? (
												<div key={idx}>{part.text}</div>
											) : (
												<div key={idx} className="prose prose-sm dark:prose-invert max-w-none">
													<ReactMarkdown>{part.text || ""}</ReactMarkdown>
												</div>
											)
										) : null
									)}
								</div>
							</div>
						))}
						{isGenerating && <div className="flex items-center gap-2 text-muted-foreground"><div className="animate-pulse">Generating...</div></div>}
						<div ref={messagesEndRef} />
					</div>

					{/* Input */}
					<div className="p-3 border-t">
						<div className="flex gap-2">
							<textarea
								value={input}
								onChange={(e) => setInput(e.target.value)}
								placeholder="Type a message..."
								className="flex-1 min-h-[60px] p-3 border rounded-md resize-none bg-background"
								onKeyDown={(e) => {
									if (e.key === "Enter" && !e.shiftKey) {
										e.preventDefault();
										handleSendMessage();
									}
								}}
							/>
							<div className="flex flex-col gap-2">
								{isGenerating ? (
									<Button onClick={handleStop} variant="secondary" size="icon">
										<Square className="h-4 w-4" />
									</Button>
								) : (
									<Button onClick={handleSendMessage} disabled={!input.trim()} size="icon">
										<Send className="h-4 w-4" />
									</Button>
								)}
							</div>
						</div>
					</div>
				</div>

				{/* Preview Panel */}
				<div className={isMobile ? `absolute inset-0 z-10 transition-transform duration-200 bg-background ${mobileActiveTab === "preview" ? "translate-x-0" : "translate-x-full"}` : "overflow-hidden h-full"} style={isMobile ? { top: "0", bottom: "calc(60px + env(safe-area-inset-bottom))" } : undefined}>
					{appInfo.gitRepo ? (
						<WebView
							repoId={appInfo.gitRepo}
							appId={appId}
							codeServerUrl={devServerUrls.codeServerUrl}
							consoleUrl={devServerUrls.consoleUrl}
							vmUrl={devServerUrls.ephemeralUrl}
						/>
					) : (
						<div className="flex items-center justify-center h-full text-muted-foreground">
							<p>Preview requires a git repository to be configured</p>
						</div>
					)}
				</div>
			</div>

			{/* Mobile Tab Navigation */}
			{isMobile && (
				<div className="fixed bottom-0 left-0 right-0 flex border-t bg-background/95 backdrop-blur-sm" style={{ paddingBottom: "env(safe-area-inset-bottom)" }}>
					<button onClick={() => setMobileActiveTab("chat")} className={`flex-1 flex flex-col items-center justify-center py-2 px-1 transition-colors ${mobileActiveTab === "chat" ? "text-primary" : "text-muted-foreground hover:text-foreground"}`}>
						<MessageCircle className={`h-6 w-6 mb-1 ${mobileActiveTab === "chat" ? "fill-current" : ""}`} />
						<span className="text-xs font-medium">Chat</span>
					</button>
					<button onClick={() => setMobileActiveTab("preview")} className={`flex-1 flex flex-col items-center justify-center py-2 px-1 transition-colors ${mobileActiveTab === "preview" ? "text-primary" : "text-muted-foreground hover:text-foreground"}`}>
						<Monitor className={`h-6 w-6 mb-1 ${mobileActiveTab === "preview" ? "fill-current" : ""}`} />
						<span className="text-xs font-medium">Preview</span>
					</button>
				</div>
			)}
		</div>
	);
}

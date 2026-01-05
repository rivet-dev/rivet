import { createClient } from "@rivetkit/react";
import { useEffect, useRef, useState } from "react";
import type {
	CursorPosition,
	TextLabel,
	registry,
} from "../backend/registry";

const rivetUrl = "http://localhost:6420";

const client = createClient<typeof registry>(rivetUrl);

// Generate a random user ID
const generateUserId = () =>
	`user-${Math.random().toString(36).substring(2, 9)}`;

// Generate a random session ID
const generateSessionId = () =>
	`session-${Math.random().toString(36).substring(2, 15)}`;

// Cursor colors for different users (darker palette)
const CURSOR_COLORS = [
	"#E63946",
	"#2A9D8F",
	"#1B8AAE",
	"#F77F00",
	"#06A77D",
	"#D4A017",
	"#9B59B6",
	"#5DADE2",
];

function getColorForUser(userId: string): string {
	let hash = 0;
	for (let i = 0; i < userId.length; i++) {
		hash = userId.charCodeAt(i) + ((hash << 5) - hash);
	}
	return CURSOR_COLORS[Math.abs(hash) % CURSOR_COLORS.length];
}

// Virtual canvas size - all coordinates are in this space
const CANVAS_WIDTH = 1920;
const CANVAS_HEIGHT = 1080;

export function App() {
	const [roomId, setRoomId] = useState("general");
	const [userId] = useState(generateUserId());
	const [sessionId] = useState(generateSessionId());
	const [cursors, setCursors] = useState<Record<string, CursorPosition>>({});
	const [textLabels, setTextLabels] = useState<TextLabel[]>([]);
	const [textInput, setTextInput] = useState("");
	const [isTyping, setIsTyping] = useState(false);
	const [typingPosition, setTypingPosition] = useState({ x: 0, y: 0 });
	const [currentTextId, setCurrentTextId] = useState<string | null>(null);
	const [scale, setScale] = useState(1);
	const [connected, setConnected] = useState(false);
	const canvasRef = useRef<HTMLDivElement>(null);
	const containerRef = useRef<HTMLDivElement>(null);
	const wsRef = useRef<WebSocket | null>(null);

	// Calculate scale factor to fit canvas in viewport
	useEffect(() => {
		const updateScale = () => {
			if (!containerRef.current) return;

			const containerWidth = containerRef.current.clientWidth;
			const containerHeight = containerRef.current.clientHeight;

			// Calculate scale to fit canvas while maintaining aspect ratio
			const scaleX = containerWidth / CANVAS_WIDTH;
			const scaleY = containerHeight / CANVAS_HEIGHT;
			const newScale = Math.min(scaleX, scaleY);

			setScale(newScale);
		};

		updateScale();
		window.addEventListener("resize", updateScale);
		return () => window.removeEventListener("resize", updateScale);
	}, []);

	// Connect to WebSocket
	useEffect(() => {
		let ws: WebSocket | null = null;

		const connect = async () => {
			try {
				// Get or create the actor for this room
				const actorId = await client.cursorRoom.getOrCreate(roomId).resolve();
				console.log("found actor", actorId);

				const wsOrigin = rivetUrl.replace(/^http/, "ws");
				const wsUrl = `${wsOrigin}/gateway/${actorId}/websocket?sessionId=${encodeURIComponent(sessionId)}`;

				console.log("ws url:", wsUrl);

				// Create WebSocket connection
				ws = new WebSocket(wsUrl);
				wsRef.current = ws;

				ws.addEventListener("open", () => {
					console.log("websocket connected");
					setConnected(true);
				});

				ws.addEventListener("message", (event) => {
					try {
						const message = JSON.parse(event.data);

						switch (message.type) {
							case "init": {
								// Initial state from server
								setCursors(message.data.cursors);
								setTextLabels(message.data.textLabels);
								break;
							}

							case "cursorMoved": {
								setCursors((prev) => ({
									...prev,
									[message.data.userId]: message.data,
								}));
								break;
							}

							case "textUpdated": {
								setTextLabels((prev) => {
									const existingIndex = prev.findIndex(
										(l) => l.id === message.data.id,
									);
									if (existingIndex >= 0) {
										const newLabels = [...prev];
										newLabels[existingIndex] = message.data;
										return newLabels;
									} else {
										return [...prev, message.data];
									}
								});
								break;
							}

							case "textRemoved": {
								setTextLabels((prev) =>
									prev.filter((label) => label.id !== message.data),
								);
								break;
							}

							case "cursorRemoved": {
								setCursors((prev) => {
									const newCursors = { ...prev };
									delete newCursors[message.data.userId];
									return newCursors;
								});
								break;
							}
						}
					} catch (error) {
						console.error("error parsing websocket message:", error);
					}
				});

				ws.addEventListener("close", () => {
					console.log("websocket disconnected");
					setConnected(false);
					wsRef.current = null;
				});

				ws.addEventListener("error", (error) => {
					console.error("websocket error:", error);
				});
			} catch (error) {
				console.error("error connecting:", error);
			}
		};

		connect();

		// Cleanup
		return () => {
			if (ws) {
				ws.close();
			}
		};
	}, [roomId, sessionId]);

	// Convert screen coordinates to canvas coordinates
	const screenToCanvas = (screenX: number, screenY: number) => {
		if (!canvasRef.current) return { x: 0, y: 0 };

		const rect = canvasRef.current.getBoundingClientRect();
		const x = (screenX - rect.left) / scale;
		const y = (screenY - rect.top) / scale;

		return { x, y };
	};

	// Handle mouse movement on canvas
	const handleMouseMove = (e: React.MouseEvent<HTMLDivElement>) => {
		if (wsRef.current && wsRef.current.readyState === WebSocket.OPEN && canvasRef.current) {
			const { x, y } = screenToCanvas(e.clientX, e.clientY);
			wsRef.current.send(
				JSON.stringify({
					type: "updateCursor",
					data: { userId, x, y },
				}),
			);
		}
	};

	// Handle canvas click
	const handleCanvasClick = (e: React.MouseEvent<HTMLDivElement>) => {
		if (!canvasRef.current) return;

		const { x, y } = screenToCanvas(e.clientX, e.clientY);
		const newTextId = `${userId}-${Date.now()}`;
		setTypingPosition({ x, y });
		setCurrentTextId(newTextId);
		setIsTyping(true);
		setTextInput("");
	};

	// Handle text input changes
	const handleTextChange = (newText: string) => {
		setTextInput(newText);
		if (wsRef.current && wsRef.current.readyState === WebSocket.OPEN && currentTextId && newText.trim()) {
			wsRef.current.send(
				JSON.stringify({
					type: "updateText",
					data: {
						id: currentTextId,
						userId,
						text: newText,
						x: typingPosition.x,
						y: typingPosition.y,
					},
				}),
			);
		}
	};

	// Handle key press while typing
	const handleKeyDown = (e: React.KeyboardEvent) => {
		if (e.key === "Enter") {
			// Finalize the text
			if (textInput.trim() && wsRef.current && wsRef.current.readyState === WebSocket.OPEN && currentTextId) {
				wsRef.current.send(
					JSON.stringify({
						type: "updateText",
						data: {
							id: currentTextId,
							userId,
							text: textInput,
							x: typingPosition.x,
							y: typingPosition.y,
						},
					}),
				);
			} else if (wsRef.current && wsRef.current.readyState === WebSocket.OPEN && currentTextId) {
				// Remove empty text
				wsRef.current.send(
					JSON.stringify({
						type: "removeText",
						data: { id: currentTextId },
					}),
				);
			}
			setTextInput("");
			setIsTyping(false);
			setCurrentTextId(null);
		} else if (e.key === "Escape") {
			// Cancel typing and remove text
			if (wsRef.current && wsRef.current.readyState === WebSocket.OPEN && currentTextId) {
				wsRef.current.send(
					JSON.stringify({
						type: "removeText",
						data: { id: currentTextId },
					}),
				);
			}
			setTextInput("");
			setIsTyping(false);
			setCurrentTextId(null);
		}
	};

	return (
		<div className="app-container">
			<div className="controls">
				<div className="control-group">
					<label>Room:</label>
					<input
						type="text"
						value={roomId}
						onChange={(e) => setRoomId(e.target.value)}
						placeholder="Enter room name"
					/>
				</div>
				<div className="user-info">
					Your ID: <span style={{ color: getColorForUser(userId) }}>{userId}</span>
				</div>
			</div>

			<div ref={containerRef} className="canvas-container">
				<div
					ref={canvasRef}
					className="canvas"
					style={{
						width: `${CANVAS_WIDTH}px`,
						height: `${CANVAS_HEIGHT}px`,
						transform: `translate(-50%, -50%) scale(${scale})`,
					}}
					onMouseMove={handleMouseMove}
					onClick={handleCanvasClick}
					tabIndex={0}
					onKeyDown={handleKeyDown}
				>
					{/* Render text labels */}
					{textLabels
						.filter((label) => label.id !== currentTextId)
						.map((label) => (
							<div
								key={label.id}
								className="text-label"
								style={{
									left: label.x,
									top: label.y,
									color: getColorForUser(label.userId),
								}}
							>
								{label.text}
							</div>
						))}

					{/* Render text being typed */}
					{isTyping && (
						<div
							className="typing-container"
							style={{
								left: typingPosition.x,
								top: typingPosition.y,
							}}
						>
							<div
								className="typing-text"
								style={{
									color: getColorForUser(userId),
								}}
							>
								{textInput}
								<span className="typing-cursor">|</span>
							</div>
							<div
								className="enter-hint"
								style={{
									borderColor: getColorForUser(userId),
									color: getColorForUser(userId),
								}}
							>
								enter
							</div>
						</div>
					)}

					{/* Render cursors */}
					{Object.entries(cursors).map(([id, cursor]) => {
						const color = getColorForUser(cursor.userId);
						const isOwnCursor = id === userId;
						return (
							<div
								key={id}
								className="cursor"
								style={{
									left: cursor.x,
									top: cursor.y,
								}}
							>
								<svg
									width="20"
									height="24"
									viewBox="0 0 20 24"
									fill="none"
									xmlns="http://www.w3.org/2000/svg"
									className="cursor-svg"
								>
									<path
										d="M10 4 L4 18 L16 18 Z"
										fill={color}
										stroke="white"
										strokeWidth="1.5"
										strokeLinecap="round"
										strokeLinejoin="round"
										transform="rotate(-45 10 12)"
									/>
								</svg>
								<div
									className="cursor-label"
									style={{
										backgroundColor: color,
										borderColor: `${color}40`,
									}}
								>
									{isOwnCursor ? "you" : cursor.userId}
								</div>
							</div>
						);
					})}

					{!connected && (
						<div className="loading-overlay">Connecting to room...</div>
					)}

					{/* Hidden input to capture typing */}
					{isTyping && (
						<input
							type="text"
							className="hidden-input"
							value={textInput}
							onChange={(e) => handleTextChange(e.target.value)}
							onBlur={() => {
								if (!textInput.trim() && wsRef.current && wsRef.current.readyState === WebSocket.OPEN && currentTextId) {
									wsRef.current.send(
										JSON.stringify({
											type: "removeText",
											data: { id: currentTextId },
										}),
									);
									setCurrentTextId(null);
								}
								setIsTyping(false);
							}}
							autoFocus
						/>
					)}
				</div>
			</div>
		</div>
	);
}

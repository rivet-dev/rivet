import { createClient } from "@rivetkit/react";
import { useEffect, useState } from "react";
import type { registry } from "../src/registry";

const rivetUrl = "http://localhost:6420";

const client = createClient<typeof registry>(rivetUrl);

// Generate a random user ID
const generateUserId = () =>
	`user-${Math.random().toString(36).substring(2, 9)}`;

// Cursor colors for different users
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

interface CursorPosition {
	userId: string;
	x: number;
	y: number;
	timestamp: number;
}

export function App() {
	const [userId] = useState(generateUserId());
	const [cursors, setCursors] = useState<Record<string, CursorPosition>>({});
	const [ws, setWs] = useState<WebSocket | null>(null);
	const [connected, setConnected] = useState(false);

	// Connect to WebSocket
	useEffect(() => {
		let websocket: WebSocket | null = null;

		const connect = async () => {
			try {
				// Get or create the actor for the room
				const actorId = await client.cursorRoom.getOrCreate("main").resolve();
				console.log("found actor", actorId);

				// Build WebSocket URL with userId query parameter
				const wsOrigin = rivetUrl.replace(/^http/, "ws");
				const wsUrl = `${wsOrigin}/gateway/${actorId}/raw/websocket?userId=${encodeURIComponent(userId)}`;

				console.log("ws url:", wsUrl);

				// Create WebSocket connection
				websocket = new WebSocket(wsUrl);

				websocket.onopen = () => {
					console.log("websocket connected");
					setConnected(true);
				};

				websocket.onmessage = (event) => {
					try {
						const message = JSON.parse(event.data);

						switch (message.type) {
							case "init": {
								// Initial state from server
								setCursors(message.data.cursors);
								break;
							}

							case "cursorUpdate": {
								// Update cursor position
								setCursors((prev) => ({
									...prev,
									[message.data.userId]: message.data,
								}));
								break;
							}

							case "cursorsState": {
								// Full cursor state
								setCursors(message.data.cursors);
								break;
							}
						}
					} catch (error) {
						console.error("error parsing websocket message:", error);
					}
				};

				websocket.onclose = () => {
					console.log("websocket disconnected");
					setConnected(false);
				};

				websocket.onerror = (error) => {
					console.error("websocket error:", error);
				};

				setWs(websocket);
			} catch (error) {
				console.error("error connecting:", error);
			}
		};

		connect();

		// Cleanup
		return () => {
			if (websocket) {
				websocket.close();
			}
		};
	}, [userId]);

	// Handle mouse movement
	const handleMouseMove = (e: React.MouseEvent<HTMLDivElement>) => {
		if (ws && ws.readyState === WebSocket.OPEN) {
			const rect = e.currentTarget.getBoundingClientRect();
			const x = e.clientX - rect.left;
			const y = e.clientY - rect.top;

			// Send cursor update via WebSocket
			ws.send(
				JSON.stringify({
					type: "updateCursor",
					data: { x, y },
				}),
			);
		}
	};

	return (
		<div className="app-container">
			<div className="header">
				<h1>Quickstart: Raw WebSockets</h1>
				<div className="info">
					<div className="connection-status">
						Status: <span className={connected ? "connected" : "disconnected"}>
							{connected ? "Connected" : "Disconnected"}
						</span>
					</div>
					<div className="user-info">
						Your ID: <span style={{ color: getColorForUser(userId) }}>{userId}</span>
					</div>
				</div>
			</div>

			<div className="canvas" onMouseMove={handleMouseMove}>
				{/* Render cursors */}
				{Object.entries(cursors).map(([id, cursor]) => {
					const color = getColorForUser(cursor.userId);
					const isOwnCursor = cursor.userId === userId;
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
			</div>
		</div>
	);
}

import React, { useState, useEffect, useRef } from "react";
import { createRivetKit } from "@rivetkit/react";
import type { registry } from "../src/actors.ts";

const { useActor } = createRivetKit<typeof registry>(`${window.location.origin}/api/rivet`);

export default function App() {
	const [messages, setMessages] = useState<Array<{ id: string; text: string; timestamp: number }>>([]);
	const [inputText, setInputText] = useState("");
	const [isConnected, setIsConnected] = useState(false);

	// Connect to the WebSocket actor
	const chatRoom = useActor({
		name: "chatRoom",
		key: ["random"],
	});

	// Raw WS we created to connect to the actors
	const wsRef = useRef<WebSocket | null>(null);

	useEffect(() => {
		(async () => {
			const ws = await chatRoom.handle?.webSocket();

			if (!ws) return;

			ws.onopen = () => {
				setIsConnected(true);
				console.log("Connected via direct access!");
			};

			ws.onmessage = (event: MessageEvent) => {
				const data = JSON.parse(event.data);

				if (data.type === "init") {
					setMessages(data.messages);
				} else if (data.type === "message") {
					setMessages(prev => [...prev, {
						id: data.id,
						text: data.text,
						timestamp: data.timestamp
					}]);
				}
			};

			ws.onclose = (event: CloseEvent) => {
				setIsConnected(false);
				console.log("WebSocket closed:", event.code, event.reason);
			};

			ws.onerror = (event: Event) => {
				console.error("WebSocket error:", event);
			};

			wsRef.current = ws;
		})();

		return () => {
			if (wsRef.current) {
				wsRef.current.close();
			}
		};
	}, [chatRoom.handle]);

	const sendMessage = (e: React.FormEvent) => {
		e.preventDefault();
		if (!inputText.trim() || !wsRef.current || wsRef.current.readyState !== WebSocket.OPEN) return;

		wsRef.current.send(JSON.stringify({
			type: "message",
			text: inputText.trim()
		}));
		setInputText("");
	};

	return (
		<div style={{ maxWidth: "800px", margin: "0 auto", padding: "20px" }}>
			<h1>Raw WebSocket Chat</h1>

			<div style={{
				padding: "10px",
				background: isConnected ? "#4caf50" : "#f44336",
				color: "white",
				borderRadius: "4px",
				marginBottom: "20px"
			}}>
				{isConnected ? "Connected" : "Disconnected"}
			</div>

			<div style={{
				background: "white",
				border: "1px solid #ddd",
				borderRadius: "8px",
				padding: "10px",
				height: "400px",
				overflowY: "auto",
				marginBottom: "10px"
			}}>
				{messages.map((msg) => (
					<div key={msg.id} style={{ marginBottom: "10px" }}>
						<strong>{new Date(msg.timestamp).toLocaleTimeString()}:</strong> {msg.text}
					</div>
				))}
			</div>

			<form onSubmit={sendMessage} style={{ display: "flex", gap: "10px" }}>
				<input
					type="text"
					value={inputText}
					onChange={(e) => setInputText(e.target.value)}
					placeholder="Type a message..."
					style={{ flex: 1, padding: "8px", borderRadius: "4px", border: "1px solid #ddd" }}
				/>
				<button type="submit">Send</button>
			</form>
		</div>
	);
}

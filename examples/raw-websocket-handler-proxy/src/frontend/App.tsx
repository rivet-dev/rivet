import React, { useState, useEffect, useRef } from "react";

export default function App() {
	const [messages, setMessages] = useState<Array<{ id: string; text: string; timestamp: number }>>([]);
	const [inputText, setInputText] = useState("");
	const [isConnected, setIsConnected] = useState(false);
	const wsRef = useRef<WebSocket | null>(null);

	useEffect(() => {
		// Direct WebSocket connection to actor
		const protocols = [
			`query.${encodeURIComponent(JSON.stringify({
				getOrCreateForKey: { name: "chatRoom", key: "main" }
			}))}`,
			`encoding.json`,
			`conn_params.${encodeURIComponent(JSON.stringify({ apiKey: "your-api-key" }))}`
		];

		const ws = new WebSocket("ws://localhost:8080/registry/actors/chatRoom/ws/", protocols);
		
		ws.onopen = () => {
			setIsConnected(true);
			console.log("Connected via direct access!");
		};

		ws.onmessage = (event) => {
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

		ws.onclose = (event) => {
			setIsConnected(false);
			console.log("WebSocket closed:", event.code, event.reason);
		};

		ws.onerror = (event) => {
			console.error("WebSocket error:", event);
		};

		wsRef.current = ws;

		return () => {
			ws.close();
		};
	}, []);

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

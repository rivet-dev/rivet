import type { UniversalWebSocket } from "rivetkit";

export function handleWebSocket(c: any, websocket: UniversalWebSocket) {
	const connectionId = `conn-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;

	// Initialize WebSocket state if not exists
	if (!c.state.connectionCount) c.state.connectionCount = 0;
	if (!c.state.messageCount) c.state.messageCount = 0;
	if (!c.state.messageHistory) c.state.messageHistory = [];

	c.state.connectionCount++;
	c.log.info("websocket connected", {
		connectionCount: c.state.connectionCount,
		connectionId,
		url: c.request.url,
	});

	// Send welcome message
	const welcomeMessage = JSON.stringify({
		type: "welcome",
		connectionId,
		connectionCount: c.state.connectionCount,
		timestamp: Date.now(),
	});

	websocket.send(welcomeMessage);
	c.state.messageHistory.push({
		type: "sent",
		data: welcomeMessage,
		timestamp: Date.now(),
		connectionId,
	});

	// Handle incoming messages
	websocket.addEventListener("message", (event: any) => {
		c.state.messageCount++;
		const timestamp = Date.now();

		c.log.info("websocket message received", {
			messageCount: c.state.messageCount,
			connectionId,
			dataType: typeof event.data,
		});

		// Record received message
		c.state.messageHistory.push({
			type: "received",
			data: event.data,
			timestamp,
			connectionId,
		});

		const data = event.data;

		if (typeof data === "string") {
			try {
				const parsed = JSON.parse(data);

				if (parsed.type === "ping") {
					const pongMessage = JSON.stringify({
						type: "pong",
						timestamp,
						originalTimestamp: parsed.timestamp,
					});
					websocket.send(pongMessage);
					c.state.messageHistory.push({
						type: "sent",
						data: pongMessage,
						timestamp,
						connectionId,
					});
				} else if (parsed.type === "echo") {
					const echoMessage = JSON.stringify({
						type: "echo-response",
						originalMessage: parsed.message,
						timestamp,
					});
					websocket.send(echoMessage);
					c.state.messageHistory.push({
						type: "sent",
						data: echoMessage,
						timestamp,
						connectionId,
					});
				} else if (parsed.type === "getStats") {
					const statsMessage = JSON.stringify({
						type: "stats",
						connectionCount: c.state.connectionCount,
						messageCount: c.state.messageCount,
						timestamp,
					});
					websocket.send(statsMessage);
					c.state.messageHistory.push({
						type: "sent",
						data: statsMessage,
						timestamp,
						connectionId,
					});
				} else if (parsed.type === "broadcast") {
					// Broadcast to all connections would need additional infrastructure
					const broadcastResponse = JSON.stringify({
						type: "broadcast-ack",
						message: parsed.message,
						timestamp,
					});
					websocket.send(broadcastResponse);
					c.state.messageHistory.push({
						type: "sent",
						data: broadcastResponse,
						timestamp,
						connectionId,
					});
				} else {
					// Echo back unknown JSON messages
					websocket.send(data);
					c.state.messageHistory.push({
						type: "sent",
						data: data,
						timestamp,
						connectionId,
					});
				}
			} catch {
				// If not JSON, just echo it back
				websocket.send(data);
				c.state.messageHistory.push({
					type: "sent",
					data: data,
					timestamp,
					connectionId,
				});
			}
		} else if (data instanceof ArrayBuffer || data instanceof Uint8Array) {
			// Handle binary data - reverse the bytes
			const bytes = new Uint8Array(data);
			const reversed = new Uint8Array(bytes.length);
			for (let i = 0; i < bytes.length; i++) {
				reversed[i] = bytes[bytes.length - 1 - i];
			}
			websocket.send(reversed);
			c.state.messageHistory.push({
				type: "sent",
				data: `[Binary: ${reversed.length} bytes - reversed]`,
				timestamp,
				connectionId,
			});
		} else {
			// Echo other data types
			websocket.send(data);
			c.state.messageHistory.push({
				type: "sent",
				data: data,
				timestamp,
				connectionId,
			});
		}
	});

	// Handle connection close
	websocket.addEventListener("close", () => {
		c.state.connectionCount--;
		c.log.info("websocket disconnected", {
			connectionCount: c.state.connectionCount,
			connectionId,
		});
	});

	// Handle errors
	websocket.addEventListener("error", (error: any) => {
		c.log.error("websocket error", { error: error.message, connectionId });
	});
}

export const websocketActions = {
	getWebSocketStats: (c: any) => {
		return {
			connectionCount: c.state.connectionCount || 0,
			messageCount: c.state.messageCount || 0,
			messageHistory: (c.state.messageHistory || []).slice(-50), // Last 50 messages
		};
	},
	clearWebSocketHistory: (c: any) => {
		c.state.messageHistory = [];
		c.state.messageCount = 0;
		return true;
	},
	getWebSocketMessageHistory: (c: any, limit: number = 20) => {
		return (c.state.messageHistory || []).slice(-limit);
	},
};

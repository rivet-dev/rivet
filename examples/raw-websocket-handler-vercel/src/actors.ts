import { actor, setup } from "rivetkit";

export const chatRoom = actor({
	state: {
		messages: [] as Array<{
			id: string;
			text: string;
			timestamp: number;
		}>,
	},
	createVars: () => {
		return {
			sockets: new Set<any>(),
		};
	},
	onWebSocket(ctx, socket) {
		// Add socket to the set
		ctx.vars.sockets.add(socket);

		// Send recent messages to new connection
		socket.send(
			JSON.stringify({
				type: "init",
				messages: ctx.state.messages,
			}),
		);

		// Handle incoming messages
		socket.addEventListener("message", (event: any) => {
			try {
				const data = JSON.parse(event.data);

				if (data.type === "message" && data.text) {
					const message = {
						id: crypto.randomUUID(),
						text: data.text,
						timestamp: Date.now(),
					};

					// Add to state
					ctx.state.messages.push(message);
					ctx.saveState({});

					// Keep only last 50 messages
					if (ctx.state.messages.length > 50) {
						ctx.state.messages.shift();
					}

					// Broadcast to all connected clients
					const broadcast = JSON.stringify({
						type: "message",
						...message,
					});

					for (const ws of ctx.vars.sockets) {
						if (ws.readyState === 1) {
							// OPEN
							ws.send(broadcast);
						}
					}
				}
			} catch (e) {
				console.error("Failed to process message:", e);
			}
		});

		// Remove socket on close
		socket.addEventListener("close", () => {
			ctx.vars.sockets.delete(socket);
		});
	},
	actions: {},
});

export const registry = setup({
	use: { chatRoom },
});

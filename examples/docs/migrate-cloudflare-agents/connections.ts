import { actor, event, UserError } from "rivetkit";

interface ChatParams {
	token: string;
}

interface Member {
	userId: string;
	joinedAt: number;
}

// Replace with your own token verification
function userIdFromToken(token: string): string | null {
	return token.length > 0 ? `user-${token.slice(0, 8)}` : null;
}

export const chatAgent = actor({
	events: {
		message: event<{ from: string; text: string }>(),
	},

	// Live connections, keyed by connection ID
	createVars: () => ({ members: new Map<string, Member>() }),

	// Reject a connection before it opens. Stateless action calls run this
	// too, with null params unless the handle was created with params.
	onBeforeConnect: (_c, params: ChatParams | null) => {
		if (!params?.token || !userIdFromToken(params.token)) {
			throw new UserError("invalid token");
		}
	},

	// Like connection.setState(): store per-connection data by conn.id
	onConnect: (c, conn) => {
		const params = conn.params as ChatParams | null;
		const userId = params ? userIdFromToken(params.token) : null;
		if (!userId) return;
		c.vars.members.set(conn.id, { userId, joinedAt: Date.now() });
	},

	// Like onClose
	onDisconnect: (c, conn) => {
		c.vars.members.delete(conn.id);
	},

	actions: {
		// Replaces onMessage: clients call an action instead of sending a raw frame
		send: (c, text: string) => {
			const from = c.vars.members.get(c.conn.id)?.userId ?? "unknown";

			// Like this.broadcast(message, [connection.id]): skip the sender
			for (const conn of c.conns.values()) {
				if (conn.id !== c.conn.id) {
					conn.send("message", { from, text });
				}
			}
		},
	},
});

import { actor, event, queue } from "rivetkit";

interface AccessControlConnParams {
	allowRequest?: boolean;
	allowWebSocket?: boolean;
	invalidCanInvokeReturn?: boolean;
}

export const accessControlActor = actor({
	state: {
		lastCanInvokeConnId: "",
	},
	events: {
		allowedEvent: event<{ value: string }>(),
		blockedEvent: event<{ value: string }>(),
	},
	queues: {
		allowedQueue: queue<{ value: string }>(),
		blockedQueue: queue<{ value: string }>(),
	},
	canInvoke: (c, invoke) => {
		c.state.lastCanInvokeConnId = c.conn.id;
		const params = c.conn.params as AccessControlConnParams | undefined;
		if (params?.invalidCanInvokeReturn) {
			return undefined as never;
		}

		if (invoke.kind === "action") {
			if (invoke.name.startsWith("allowed")) {
				return true;
			}
			return false;
		}

		if (invoke.kind === "queue") {
			if (invoke.name === "allowedQueue") {
				return true;
			}
			return false;
		}

		if (invoke.kind === "subscribe") {
			if (invoke.name === "allowedEvent") {
				return true;
			}
			return false;
		}

		if (invoke.kind === "request") {
			if (params?.allowRequest === true) {
				return true;
			}
			return false;
		}

		if (invoke.kind === "websocket") {
			if (params?.allowWebSocket === true) {
				return true;
			}
			return false;
		}

		return false;
	},
	onRequest(_c, request) {
		const url = new URL(request.url);
		if (url.pathname === "/status") {
			return Response.json({ ok: true });
		}
		return new Response("Not Found", { status: 404 });
	},
	onWebSocket(_c, websocket) {
		websocket.send(JSON.stringify({ type: "welcome" }));
	},
	actions: {
		allowedAction: (_c, value: string) => {
			return `allowed:${value}`;
		},
		blockedAction: () => {
			return "blocked";
		},
		allowedGetLastCanInvokeConnId: (c) => {
			return c.state.lastCanInvokeConnId;
		},
		allowedReceiveQueue: async (c) => {
			const [message] = await c.queue.tryNext({
				names: ["allowedQueue"],
			});
			return message?.body ?? null;
		},
		allowedBroadcastAllowedEvent: (c, value: string) => {
			c.broadcast("allowedEvent", { value });
		},
	},
});

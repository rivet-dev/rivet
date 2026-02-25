import { actor, event, queue } from "rivetkit";
import { Forbidden } from "rivetkit/errors";

export interface AccessControlConnParams {
	allowRequest?: boolean;
	allowWebSocket?: boolean;
}

const accessControlEvents: Record<
	string,
	ReturnType<typeof event<{ value: string }>>
> = {
	allowedEvent: event<{ value: string }>({
		canSubscribe: (c) => {
			c.state.lastCanSubscribeConnId = c.conn.id;
			return true;
		},
	}),
	blockedEvent: event<{ value: string }>({
		canSubscribe: (c) => {
			c.state.lastCanSubscribeConnId = c.conn.id;
			return false;
		},
	}),
};

const accessControlQueues: Record<
	string,
	ReturnType<typeof queue<{ value: string }>>
> = {
	allowedQueue: queue<{ value: string }>({
		canPublish: (c) => {
			c.state.lastCanPublishConnId = c.conn.id;
			return true;
		},
	}),
	blockedQueue: queue<{ value: string }>({
		canPublish: (c) => {
			c.state.lastCanPublishConnId = c.conn.id;
			return false;
		},
	}),
};

export const accessControlActor = actor({
	state: {
		lastCanPublishConnId: "",
		lastCanSubscribeConnId: "",
	},
	events: accessControlEvents,
	queues: accessControlQueues,
	onBeforeConnect: (_c, params: AccessControlConnParams) => {
		if (params?.allowRequest === false || params?.allowWebSocket === false) {
			throw new Forbidden();
		}
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
		allowedGetLastCanPublishConnId: (c) => {
			return c.state.lastCanPublishConnId;
		},
		allowedGetLastCanSubscribeConnId: (c) => {
			return c.state.lastCanSubscribeConnId;
		},
		allowedReceiveQueue: async (c) => {
			const message = await c.queue.tryNext({
				names: ["allowedQueue"],
			});
			return message?.body ?? null;
		},
		allowedReceiveAnyQueue: async (c) => {
			const message = await c.queue.tryNext();
			return message?.body ?? null;
		},
		allowedBroadcastAllowedEvent: (c, value: string) => {
			c.broadcast("allowedEvent", { value });
		},
		allowedBroadcastBlockedEvent: (c, value: string) => {
			c.broadcast("blockedEvent", { value });
		},
		allowedBroadcastUndefinedEvent: (c, value: string) => {
			c.broadcast("undefinedEvent", { value });
		},
	},
});

export const accessControlNoQueuesActor = actor({
	state: {},
	actions: {
		readAnyQueue: async (c) => {
			const message = await c.queue.tryNext();
			return message?.body ?? null;
		},
	},
});

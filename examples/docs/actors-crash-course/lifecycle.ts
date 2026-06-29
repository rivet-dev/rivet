import { actor, event, queue } from "rivetkit";

interface RoomState {
	users: Record<string, boolean>;
	name?: string;
}

interface RoomInput {
	roomName: string;
}

interface ConnState {
	userId: string;
	joinedAt: number;
}

const chatRoom = actor({
	events: {
		stateChanged: event<RoomState>(),
	},
	queues: {
		work: queue<{ task: string }>(),
	},

	// State & vars initialization
	createState: (c, input: RoomInput): RoomState => ({
		users: {},
		name: input.roomName,
	}),
	createVars: () => ({ startTime: Date.now() }),

	// Actor lifecycle
	onCreate: (c) => console.log("created", c.key),
	onDestroy: (c) => console.log("destroyed"),
	onWake: (c) => console.log("actor started"),
	onSleep: (c) => console.log("actor sleeping"),
	run: async (c) => {
		for await (const message of c.queue.iter()) {
			console.log("processing", message.body.task);
		}
	},
	onStateChange: (c, newState) => c.broadcast("stateChanged", newState),

	// Connection lifecycle
	createConnState: (c, params): ConnState => ({
		userId: (params as { userId: string }).userId,
		joinedAt: Date.now(),
	}),
	onBeforeConnect: (c, params) => {
		/* validate auth */
	},
	onConnect: (c, conn) => console.log("connected:", (conn.state as ConnState).userId),
	onDisconnect: (c, conn) => console.log("disconnected:", (conn.state as ConnState).userId),

	// Networking
	onRequest: (c, req) => new Response(JSON.stringify(c.state)),
	onWebSocket: (c, socket) => socket.addEventListener("message", console.log),

	// Response transformation
	onBeforeActionResponse: <Out>(
		c: unknown,
		name: string,
		args: unknown[],
		output: Out,
	): Out => output,

	actions: {},
});

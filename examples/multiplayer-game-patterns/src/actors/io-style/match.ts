import { actor } from "rivetkit";
import { err, sleep } from "../../utils.ts";
import { buildSecret } from "../shared/ids.ts";

interface Player {
	playerId: string;
	name: string;
	joinedAt: number;
}

interface State {
	matchId: string;
	trusted: boolean;
	capacity: number;
	tickMs: number;
	tick: number;
	phase: "lobby" | "live";
	players: Record<string, Player>;
	playerTokens: Record<string, string>;
}

function requireTrusted(state: State) {
	if (!state.trusted) {
		err("match is not trusted", "untrusted_lobby");
	}
}

function buildSnapshot(state: State) {
	// Snapshots expose scaffold state to clients without gameplay internals.
	return {
		matchId: state.matchId,
		capacity: state.capacity,
		tick: state.tick,
		phase: state.phase,
		players: state.players,
		playerCount: Object.keys(state.players).length,
	};
}

async function syncRoomIndex(c: {
	state: State;
	client: <T>() => any;
}) {
	// The matchmaker stores room index rows in SQLite.
	// This heartbeat call keeps that index in sync with in-memory player counts.
	const client = c.client<any>();
	await client.ioStyleMatchmaker.getOrCreate(["main"]).queue.roomHeartbeat.send({
		matchId: c.state.matchId,
		playerCount: Object.keys(c.state.players).length,
		capacity: c.state.capacity,
	});
}

export const ioStyleMatch = actor({
	createState: (
		_c,
		input: { matchId: string; capacity: number; tickMs: number },
	): State => ({
		matchId: input.matchId,
		trusted: true,
		capacity: input.capacity,
		tickMs: input.tickMs,
		tick: 0,
		phase: "lobby",
		players: {},
		playerTokens: {},
	}),
	createConnState: (_c, _params: { playerToken?: string }) => ({
		playerId: null as string | null,
	}),
	onBeforeConnect: (c, params: { playerToken?: string }) => {
		const playerToken = params?.playerToken?.trim();
		if (!playerToken) {
			return;
		}
		const playerId = c.state.playerTokens[playerToken];
		if (!playerId) {
			err("invalid player token", "invalid_player_token");
		}
	},
	onConnect: async (c, conn) => {
		const playerToken = conn.params?.playerToken?.trim();
		if (!playerToken) {
			conn.disconnect("invalid_player_token");
			return;
		}
		const playerId = c.state.playerTokens[playerToken];
		if (!playerId) {
			conn.disconnect("invalid_player_token");
			return;
		}

		if (!c.state.players[playerId]) {
			if (Object.keys(c.state.players).length >= c.state.capacity) {
				conn.disconnect("room_full");
				return;
			}
			c.state.players[playerId] = {
				playerId,
				name: playerId,
				joinedAt: Date.now(),
			};
		}

		conn.state.playerId = playerId;
		c.state.phase = "live";
		await syncRoomIndex(c);
		c.broadcast("snapshot", buildSnapshot(c.state));
	},
	onWake: async (c) => {
		// Publish the initial room row when this actor wakes.
		await syncRoomIndex(c);
	},
	onDestroy: async (c) => {
		const client = c.client<any>();
		try {
			await client.ioStyleMatchmaker.getOrCreate(["main"]).queue.roomClosed.send({
				matchId: c.state.matchId,
			});
		} catch {
			// Best effort during shutdown.
		}
	},
	onDisconnect: async (c, conn) => {
		const playerId = conn.state.playerId;
		if (!playerId) return;
		if (!c.state.players[playerId]) return;

		delete c.state.players[playerId];
		conn.state.playerId = null;
		c.state.phase = Object.keys(c.state.players).length > 0 ? "live" : "lobby";
		try {
			await syncRoomIndex(c);
		} catch {
			// Best effort during teardown.
		}
		if (Object.keys(c.state.players).length === 0) {
			c.destroy();
			return;
		}
		try {
			c.broadcast("snapshot", buildSnapshot(c.state));
		} catch {
			// Best effort during teardown.
		}
	},
	run: async (c) => {
		// This run loop is the 10 tps scaffold tick for io-style realtime sessions.
		while (!c.aborted) {
			await sleep(c.state.tickMs);
			if (c.aborted) break;
			c.state.tick += 1;
			c.state.phase = Object.keys(c.state.players).length > 0 ? "live" : "lobby";
			// Broadcast a lightweight tick snapshot to all connected players.
			c.broadcast("tick", buildSnapshot(c.state));
		}
	},
	actions: {
		issuePlayerToken: (
			c,
			input: { playerId: string },
		) => {
			const playerToken = buildSecret();
			c.state.playerTokens[playerToken] = input.playerId;
			return { playerId: input.playerId, playerToken };
		},
		getSnapshot: (c) => buildSnapshot(c.state),
	},
});

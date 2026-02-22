import { actor, type ActorContextOf, event, UserError } from "rivetkit";
import { interval } from "rivetkit/utils";
import { INTERNAL_TOKEN } from "../../auth.ts";
import { registry } from "../index.ts";
import { CAPACITY, SPEED, WORLD_SIZE } from "./config.ts";

const TICK_MS = 100;
const DISCONNECT_GRACE_MS = 5000;

interface PlayerEntry {
	token: string;
	connId: string | null;
	x: number;
	y: number;
	inputX: number;
	inputY: number;
	disconnectedAt: number | null;
}

interface State {
	matchId: string;
	tick: number;
	players: Record<string, PlayerEntry>;
}

export const ioStyleMatch = actor({
	options: { name: "IO - Match", icon: "earth-americas" },
	events: {
		snapshot: event<Snapshot>(),
	},
	createState: (_c, input: { matchId: string }): State => ({
		matchId: input.matchId,
		tick: 0,
		players: {},
	}),
	onBeforeConnect: (
		c,
		params: { playerToken?: string; internalToken?: string },
	) => {
		if (params?.internalToken === INTERNAL_TOKEN) return;
		const playerToken = params?.playerToken?.trim();
		if (!playerToken) {
			throw new UserError("authentication required", {
				code: "auth_required",
			});
		}
		if (!findPlayerByToken(c.state, playerToken)) {
			throw new UserError("invalid player token", {
				code: "invalid_player_token",
			});
		}
	},
	onConnect: async (c, conn) => {
		const playerToken = conn.params?.playerToken?.trim();
		if (!playerToken) return;
		const found = findPlayerByToken(c.state, playerToken);
		if (!found) {
			conn.disconnect("invalid_player_token");
			return;
		}
		const [, player] = found;
		player.connId = conn.id;
		player.disconnectedAt = null;

		await updateMatchmaker(c);
		broadcastSnapshot(c);
	},
	onDestroy: async (c) => {
		const client = c.client<typeof registry>();
		await client.ioStyleMatchmaker
			.getOrCreate(["main"])
			.send("closeMatch", {
				matchId: c.state.matchId,
			});
	},
	onDisconnect: async (c, conn) => {
		const found = findPlayerByConnId(c.state, conn.id);
		if (!found) return;
		const [, player] = found;
		player.connId = null;
		player.disconnectedAt = Date.now();

		await updateMatchmaker(c);
		broadcastSnapshot(c);
	},
	run: async (c) => {
		const tick = interval(TICK_MS);
		while (!c.aborted) {
			await tick();
			if (c.aborted) break;
			c.state.tick += 1;

			const now = Date.now();
			for (const [id, player] of Object.entries(c.state.players)) {
				// Remove players that disconnected beyond the grace period.
				if (
					player.disconnectedAt &&
					now - player.disconnectedAt > DISCONNECT_GRACE_MS
				) {
					delete c.state.players[id];
					continue;
				}

				player.x = Math.max(
					0,
					Math.min(WORLD_SIZE, player.x + SPEED * player.inputX),
				);
				player.y = Math.max(
					0,
					Math.min(WORLD_SIZE, player.y + SPEED * player.inputY),
				);
			}

			broadcastSnapshot(c);
		}
	},
	actions: {
		// Called by matchmaker to add a player
		createPlayer: (c, input: { playerId: string; playerToken: string }) => {
			c.state.players[input.playerId] = {
				token: input.playerToken,
				connId: null,
				x: Math.random() * WORLD_SIZE,
				y: Math.random() * WORLD_SIZE,
				inputX: 0,
				inputY: 0,
				disconnectedAt: Date.now(),
			};
		},
		// Called by player to update state
		setInput: (c, input: { inputX: number; inputY: number }) => {
			const found = findPlayerByConnId(c.state, c.conn.id);
			if (!found) {
				throw new UserError("player not found", {
					code: "player_not_found",
				});
			}
			const [, player] = found;
			player.inputX = Math.max(-1, Math.min(1, input.inputX));
			player.inputY = Math.max(-1, Math.min(1, input.inputY));
		},
		getSnapshot: (c) => buildSnapshot(c),
	},
});

function broadcastSnapshot(c: ActorContextOf<typeof ioStyleMatch>) {
	c.broadcast("snapshot", buildSnapshot(c));
}

async function updateMatchmaker(c: ActorContextOf<typeof ioStyleMatch>) {
	const client = c.client<typeof registry>();
	await client.ioStyleMatchmaker.getOrCreate(["main"]).send("updateMatch", {
		matchId: c.state.matchId,
		playerCount: activePlayerCount(c.state),
	});
}

interface Snapshot {
	matchId: string;
	capacity: number;
	tick: number;
	playerCount: number;
	worldSize: number;
	players: Record<string, { x: number; y: number }>;
}

function buildSnapshot(c: ActorContextOf<typeof ioStyleMatch>): Snapshot {
	const players: Record<string, { x: number; y: number }> = {};
	for (const [id, entry] of Object.entries(c.state.players)) {
		if (entry.disconnectedAt) continue;
		players[id] = { x: entry.x, y: entry.y };
	}
	return {
		matchId: c.state.matchId,
		capacity: CAPACITY,
		tick: c.state.tick,
		playerCount: activePlayerCount(c.state),
		worldSize: WORLD_SIZE,
		players,
	};
}

function findPlayerByToken(
	state: State,
	token: string,
): [string, PlayerEntry] | null {
	for (const [id, entry] of Object.entries(state.players)) {
		if (entry.token === token) return [id, entry];
	}
	return null;
}

function findPlayerByConnId(
	state: State,
	connId: string,
): [string, PlayerEntry] | null {
	for (const [id, entry] of Object.entries(state.players)) {
		if (entry.connId === connId) return [id, entry];
	}
	return null;
}

function activePlayerCount(state: State): number {
	let count = 0;
	for (const player of Object.values(state.players)) {
		if (!player.disconnectedAt) count++;
	}
	return count;
}

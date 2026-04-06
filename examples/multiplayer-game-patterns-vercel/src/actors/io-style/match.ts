import { actor, type ActorContextOf, event, UserError } from "rivetkit";
import { interval } from "rivetkit/utils";
import { registry } from "../index.ts";
import { getPlayerColor } from "../player-color.ts";
import { CAPACITY, SPEED, WORLD_SIZE } from "./config.ts";

const TICK_MS = 100;
const DISCONNECT_GRACE_MS = 5000;

interface PlayerEntry {
	connId: string | null;
	color: string;
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

interface ConnParams {
	playerId?: string;
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
	onConnect: async (c, conn) => {
		const params = (conn.params as ConnParams | null) ?? {};
		const playerId = params.playerId;
		if (!playerId) {
			conn.disconnect("invalid_player");
			return;
		}
		const existingPlayer = c.state.players[playerId];
		if (existingPlayer) {
			existingPlayer.connId = conn.id;
			existingPlayer.disconnectedAt = null;
		} else {
			const confirmed = await claimPendingPlayer(c, playerId);
			if (!confirmed) {
				conn.disconnect("player_not_pending");
				return;
			}
			c.state.players[playerId] = {
				connId: conn.id,
				color: getPlayerColor(playerId),
				x: Math.random() * WORLD_SIZE,
				y: Math.random() * WORLD_SIZE,
				inputX: 0,
				inputY: 0,
				disconnectedAt: null,
			};
		}

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
			let removedPlayer = false;
			for (const [id, player] of Object.entries(c.state.players)) {
				if (
					player.disconnectedAt &&
					now - player.disconnectedAt > DISCONNECT_GRACE_MS
				) {
					delete c.state.players[id];
					removedPlayer = true;
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
			if (removedPlayer) {
				await updateMatchmaker(c);
			}

			broadcastSnapshot(c);
		}
	},
	actions: {
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
	await client.ioStyleMatchmaker
		.getOrCreate(["main"])
		.send("updateMatch", {
			matchId: c.state.matchId,
			connectedPlayerCount: occupiedPlayerCount(c.state),
		});
}

async function claimPendingPlayer(
	c: ActorContextOf<typeof ioStyleMatch>,
	playerId: string,
): Promise<boolean> {
	const client = c.client<typeof registry>();
	const result = await client.ioStyleMatchmaker
		.getOrCreate(["main"])
		.send(
			"pendingPlayerConnected",
			{
				matchId: c.state.matchId,
				playerId,
			},
			{ wait: true, timeout: 3_000 },
		);
	if (result.status !== "completed") {
		return false;
	}
	const response = (result as { response?: { accepted?: boolean } }).response;
	return response?.accepted === true;
}

interface Snapshot {
	matchId: string;
	capacity: number;
	tick: number;
	playerCount: number;
	worldSize: number;
	players: Record<string, { x: number; y: number; color: string }>;
}

function buildSnapshot(c: ActorContextOf<typeof ioStyleMatch>): Snapshot {
	const players: Record<string, { x: number; y: number; color: string }> = {};
	for (const [id, entry] of Object.entries(c.state.players)) {
		if (entry.disconnectedAt) continue;
		players[id] = { x: entry.x, y: entry.y, color: entry.color };
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

function occupiedPlayerCount(state: State): number {
	return Object.keys(state.players).length;
}

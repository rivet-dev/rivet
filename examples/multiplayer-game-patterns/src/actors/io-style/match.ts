import { type ActorContextOf, actor, event, UserError } from "rivetkit";
import { db } from "rivetkit/db";
import { interval } from "rivetkit/utils";
import type { registry } from "../index.ts";
import { getPlayerColor } from "../player-color.ts";
import { CAPACITY, SPEED, WORLD_SIZE } from "./config.ts";

const TICK_MS = 100;
const SNAPSHOT_INTERVAL_MS = 1_000; // Saving this often bounds how much progress a crash can lose.
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

interface MatchVars {
	matchId: string;
	tick: number;
	players: Record<string, PlayerEntry>;
	// `onWake` stores its loop here so `onSleep` can wait for it.
	loop: Promise<void> | null;
}

interface ConnParams {
	playerId?: string;
}

type PlayerRow = {
	player_id: string;
	x: number;
	y: number;
};

export const ioStyleMatch = actor({
	options: { name: "IO - Match", icon: "earth-americas" },
	events: {
		snapshot: event<Snapshot>(),
	},
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS match_info (
					id INTEGER PRIMARY KEY CHECK (id = 1),
					tick INTEGER NOT NULL
				)
			`);
			await db.execute(`
				CREATE TABLE IF NOT EXISTS players (
					player_id TEXT PRIMARY KEY,
					x REAL NOT NULL,
					y REAL NOT NULL
				)
			`);
		},
	}),
	// The match runs in memory and starts from the last save. Its key is the
	// match id.
	createVars: async (c): Promise<MatchVars> => {
		const [info] = await c.db.execute<{ tick: number }>(
			"SELECT tick FROM match_info WHERE id = 1",
		);
		const rows = await c.db.execute<PlayerRow>(
			"SELECT player_id, x, y FROM players",
		);

		// No connection survives a restart, so every claimed player starts
		// disconnected and is dropped if they don't reconnect in time.
		const players: Record<string, PlayerEntry> = {};
		for (const row of rows) {
			players[row.player_id] = {
				connId: null,
				color: getPlayerColor(row.player_id),
				x: row.x,
				y: row.y,
				inputX: 0,
				inputY: 0,
				disconnectedAt: Date.now(),
			};
		}
		return {
			matchId: c.key[0]!,
			tick: info?.tick ?? 0,
			players,
			loop: null,
		};
	},
	// docs:start connect
	onConnect: async (c, conn) => {
		const params = (conn.params as ConnParams | null) ?? {};
		const playerId = params.playerId;
		if (!playerId) {
			conn.disconnect("invalid_player");
			return;
		}
		const existingPlayer = c.vars.players[playerId];
		if (existingPlayer) {
			existingPlayer.connId = conn.id;
			existingPlayer.disconnectedAt = null;
		} else {
			const confirmed = await claimPendingPlayer(c, playerId);
			if (!confirmed) {
				conn.disconnect("player_not_pending");
				return;
			}
			c.vars.players[playerId] = {
				connId: conn.id,
				color: getPlayerColor(playerId),
				x: Math.random() * WORLD_SIZE,
				y: Math.random() * WORLD_SIZE,
				inputX: 0,
				inputY: 0,
				disconnectedAt: null,
			};

			// The matchmaker has used up this player's reservation, so save
			// the player now. A crash before the next save would otherwise
			// turn them away with player_not_pending when they reconnect.
			await saveMatch(c);
		}

		await updateMatchmaker(c);
		broadcastSnapshot(c);
	},
	// docs:end connect
	onDestroy: async (c) => {
		// `c.abortSignal` also fires on destroy. Let the loop finish its last
		// save before the database closes.
		await c.vars.loop;
		const client = c.client<typeof registry>();
		await client.ioStyleMatchmaker
			.getOrCreate(["main"])
			.closeMatch({ matchId: c.vars.matchId });
	},
	onDisconnect: async (c, conn) => {
		const found = findPlayerByConnId(c.vars, conn.id);
		if (!found) return;
		const [, player] = found;
		player.connId = null;
		player.disconnectedAt = Date.now();

		await updateMatchmaker(c);
		broadcastSnapshot(c);
	},
	// docs:start tick
	// `onWake` runs on every start, so the loop restarts after a sleep or crash
	onWake: (c) => {
		c.vars.loop = runMatch(c).catch((error) => {
			c.log.error({ msg: "match loop failed", error });
		});
	},
	// `c.abortSignal` fires on shutdown, so the loop saves and exits on its own
	onSleep: async (c) => {
		await c.vars.loop;
	},
	// docs:end tick
	actions: {
		// docs:start tick
		setInput: (c, input: { inputX: number; inputY: number }) => {
			const found = findPlayerByConnId(c.vars, c.conn.id);
			if (!found) {
				throw new UserError("player not found", {
					code: "player_not_found",
				});
			}
			const [, player] = found;
			player.inputX = Math.max(-1, Math.min(1, input.inputX));
			player.inputY = Math.max(-1, Math.min(1, input.inputY));
		},
		// docs:end tick
		getSnapshot: (c) => buildSnapshot(c),
	},
});

function broadcastSnapshot(c: ActorContextOf<typeof ioStyleMatch>) {
	c.broadcast("snapshot", buildSnapshot(c));
}

// docs:start tick
/** Steps the match every `TICK_MS` until the actor starts shutting down. */
async function runMatch(c: ActorContextOf<typeof ioStyleMatch>): Promise<void> {
	const tick = interval(TICK_MS);
	let savedAt = Date.now();
	while (!c.aborted) {
		await tick();
		if (c.aborted) break;
		c.vars.tick += 1;

		// The grace period is much shorter than the idle timeout, so an empty
		// match removes its last players here before it goes to sleep.
		const now = Date.now();
		let removedPlayer = false;
		for (const [id, player] of Object.entries(c.vars.players)) {
			if (
				player.disconnectedAt &&
				now - player.disconnectedAt > DISCONNECT_GRACE_MS
			) {
				delete c.vars.players[id];
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

		// Ticks only change memory. Save to SQLite every
		// SNAPSHOT_INTERVAL_MS instead of on every tick.
		if (now - savedAt >= SNAPSHOT_INTERVAL_MS) {
			await saveMatch(c);
			savedAt = now;
		}
	}

	// Save once more when the Actor stops, so a clean restart loses nothing.
	await saveMatch(c);
}
// docs:end tick

// docs:start save
/**
 * Writes the tick and every claimed player's position to SQLite in one
 * transaction, so a crash leaves either the previous save or this one, never
 * part of both.
 */
async function saveMatch(c: ActorContextOf<typeof ioStyleMatch>) {
	// Copy the values first, because ticks can run while the transaction
	// waits on SQLite.
	const tick = c.vars.tick;
	const players: PlayerRow[] = Object.entries(c.vars.players).map(
		([playerId, player]) => ({
			player_id: playerId,
			x: player.x,
			y: player.y,
		}),
	);

	await c.db.transaction(
		async (tx) => {
			await tx.execute(
				"INSERT OR REPLACE INTO match_info (id, tick) VALUES (1, ?)",
				tick,
			);
			await tx.execute("DELETE FROM players");
			for (const player of players) {
				await tx.execute(
					"INSERT INTO players (player_id, x, y) VALUES (?, ?, ?)",
					player.player_id,
					player.x,
					player.y,
				);
			}
		},
		{ name: "save-match" },
	);
}
// docs:end save

/**
 * Tells the matchmaker how full this match is. The count only steers new
 * players to a lobby, and the next join or leave sends it again, so a failed
 * update is logged instead of stopping the caller or the tick loop.
 */
async function updateMatchmaker(c: ActorContextOf<typeof ioStyleMatch>) {
	const client = c.client<typeof registry>();
	try {
		await client.ioStyleMatchmaker.getOrCreate(["main"]).updateMatch({
			matchId: c.vars.matchId,
			connectedPlayerCount: occupiedPlayerCount(c.vars),
		});
	} catch (error) {
		c.log.warn({ msg: "updating the matchmaker failed", error });
	}
}

// docs:start connect
async function claimPendingPlayer(
	c: ActorContextOf<typeof ioStyleMatch>,
	playerId: string,
): Promise<boolean> {
	const client = c.client<typeof registry>();
	const result = await client.ioStyleMatchmaker
		.getOrCreate(["main"])
		.pendingPlayerConnected({ matchId: c.vars.matchId, playerId });
	return result.accepted;
}
// docs:end connect

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
	for (const [id, entry] of Object.entries(c.vars.players)) {
		if (entry.disconnectedAt) continue;
		players[id] = { x: entry.x, y: entry.y, color: entry.color };
	}
	return {
		matchId: c.vars.matchId,
		capacity: CAPACITY,
		tick: c.vars.tick,
		playerCount: activePlayerCount(c.vars),
		worldSize: WORLD_SIZE,
		players,
	};
}

function findPlayerByConnId(
	vars: MatchVars,
	connId: string,
): [string, PlayerEntry] | null {
	for (const [id, entry] of Object.entries(vars.players)) {
		if (entry.connId === connId) return [id, entry];
	}
	return null;
}

function activePlayerCount(vars: MatchVars): number {
	let count = 0;
	for (const player of Object.values(vars.players)) {
		if (!player.disconnectedAt) count++;
	}
	return count;
}

function occupiedPlayerCount(vars: MatchVars): number {
	return Object.keys(vars.players).length;
}

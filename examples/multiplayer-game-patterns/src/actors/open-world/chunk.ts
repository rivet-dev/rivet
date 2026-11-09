import { type ActorContextOf, actor, event } from "rivetkit";
import { db, type RawAccess } from "rivetkit/db";
import { interval } from "rivetkit/utils";
import { getPlayerColor } from "../player-color.ts";
import {
	CHUNK_SIZE,
	SNAPSHOT_INTERVAL_MS,
	SPEED,
	SPRINT_MULTIPLIER,
	TICK_MS,
} from "./config.ts";

const GRID_COLS = Math.floor(CHUNK_SIZE / 50);
const RECONNECT_GRACE_MS = 5000;

interface PlayerEntry {
	connId: string | null;
	name: string;
	color: string;
	x: number;
	y: number;
	inputX: number;
	inputY: number;
	sprint: boolean;
	// Set when a restart drops the player's connection, and cleared when the
	// same player ID connects again.
	disconnectedAt: number | null;
}

type PlayerRow = {
	player_id: string;
	name: string;
	color: string;
	x: number;
	y: number;
};

interface ConnectionMeta {
	playerId: string;
	inChunk: boolean;
}

interface ChunkVars {
	worldId: string;
	chunkX: number;
	chunkY: number;
	tick: number;
	connections: Record<string, ConnectionMeta>;
	players: Record<string, PlayerEntry>;
	blocks: Record<string, string>;
	// `onWake` stores its tick loop here so `onSleep` and `onDestroy` can wait
	// for it.
	loop: Promise<void> | null;
}

// The open world is partitioned into fixed size chunks keyed by world ID and chunk coordinates.
// Clients can keep multiple chunk connections open to observe nearby chunks.
// Each connection carries a stable player ID while explicit enter/leave actions control chunk membership.
export const openWorldChunk = actor({
	options: { name: "Open World - Chunk", icon: "map" },
	// docs:start chunk
	events: {
		snapshot: event<Snapshot>(),
	},
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS blocks (
					grid_x INTEGER NOT NULL,
					grid_y INTEGER NOT NULL,
					color TEXT NOT NULL,
					PRIMARY KEY (grid_x, grid_y)
				)
			`);
			await db.execute(`
				CREATE TABLE IF NOT EXISTS chunk (
					id INTEGER PRIMARY KEY CHECK (id = 1),
					tick INTEGER NOT NULL
				)
			`);
			await db.execute(`
				CREATE TABLE IF NOT EXISTS players (
					player_id TEXT PRIMARY KEY,
					name TEXT NOT NULL,
					color TEXT NOT NULL,
					x REAL NOT NULL,
					y REAL NOT NULL
				)
			`);
		},
	}),
	createVars: async (c): Promise<ChunkVars> => {
		const chunkX = Number(c.key[1] ?? "0");
		const chunkY = Number(c.key[2] ?? "0");

		// Blocks are saved in SQLite. Load them into memory on every start.
		const rows = await c.db.execute<{
			grid_x: number;
			grid_y: number;
			color: string;
		}>("SELECT grid_x, grid_y, color FROM blocks");
		const blocks: Record<string, string> = {};
		for (const row of rows) {
			blocks[`${row.grid_x},${row.grid_y}`] = row.color;
		}

		// The tick loop saves the tick and players periodically. Restore them
		// without their old connections, which did not survive the restart.
		const [savedChunk] = await c.db.execute<{ tick: number }>(
			"SELECT tick FROM chunk WHERE id = 1",
		);
		const savedPlayers = await c.db.execute<PlayerRow>(
			"SELECT player_id, name, color, x, y FROM players",
		);
		const players: Record<string, PlayerEntry> = {};
		for (const row of savedPlayers) {
			players[row.player_id] = {
				connId: null,
				name: row.name,
				color: row.color,
				x: row.x,
				y: row.y,
				inputX: 0,
				inputY: 0,
				sprint: false,
				disconnectedAt: Date.now(),
			};
		}

		return {
			worldId: c.key[0] ?? "",
			chunkX: Number.isFinite(chunkX) ? chunkX : 0,
			chunkY: Number.isFinite(chunkY) ? chunkY : 0,
			tick: savedChunk?.tick ?? 0,
			connections: {},
			players,
			blocks,
			loop: null,
		};
	},
	onConnect: (c, conn) => {
		const playerId = parsePlayerId(conn.params) ?? conn.id;
		const meta: ConnectionMeta = { playerId, inChunk: false };
		c.vars.connections[conn.id] = meta;

		// A player restored after a restart is waiting for its client to
		// reconnect. Hand it back to the first connection with its ID.
		const restored = c.vars.players[playerId];
		if (restored && restored.connId === null) {
			restored.connId = conn.id;
			restored.disconnectedAt = null;
			meta.inChunk = true;
		}
		broadcastSnapshot(c);
	},
	onDisconnect: (c, conn) => {
		const meta = c.vars.connections[conn.id];
		if (!meta) return;
		delete c.vars.connections[conn.id];

		const player = c.vars.players[meta.playerId];
		if (player?.connId === conn.id) {
			delete c.vars.players[meta.playerId];
		}
		broadcastSnapshot(c);
	},
	// docs:end chunk
	// docs:start tick
	// `onWake` runs on every start, so the loop restarts after a sleep or crash.
	onWake: (c) => {
		c.vars.loop = runChunk(c).catch((error) => {
			c.log.error({ msg: "chunk loop failed", error });
		});
	},
	// `c.abortSignal` fires on shutdown, so the loop saves and exits on its own.
	onSleep: async (c) => {
		await c.vars.loop;
	},
	// The database closes right after `onDestroy`, so wait for the final save.
	onDestroy: async (c) => {
		await c.vars.loop;
	},
	// docs:end tick
	actions: {
		// docs:start membership
		enterChunk: (
			c,
			input: { name: string; spawnX?: number; spawnY?: number },
		) => {
			const meta = getConnectionMeta(c, c.conn.id);
			if (!meta) return;

			const nameRaw = input.name.trim();
			const name = nameRaw ? nameRaw.slice(0, 24) : "Player";
			const existing = c.vars.players[meta.playerId];
			c.vars.players[meta.playerId] = {
				connId: c.conn.id,
				name,
				color: existing?.color ?? getPlayerColor(meta.playerId),
				x: clampToChunk(input.spawnX) ?? CHUNK_SIZE / 2,
				y: clampToChunk(input.spawnY) ?? CHUNK_SIZE / 2,
				inputX: 0,
				inputY: 0,
				sprint: false,
				disconnectedAt: null,
			};
			meta.inChunk = true;
			broadcastSnapshot(c);
		},
		addPlayer: (
			c,
			input: { name: string; spawnX?: number; spawnY?: number },
		) => {
			const meta = getConnectionMeta(c, c.conn.id);
			if (!meta) return;

			const nameRaw = input.name.trim();
			const name = nameRaw ? nameRaw.slice(0, 24) : "Player";
			const existing = c.vars.players[meta.playerId];
			c.vars.players[meta.playerId] = {
				connId: c.conn.id,
				name,
				color: existing?.color ?? getPlayerColor(meta.playerId),
				x: clampToChunk(input.spawnX) ?? CHUNK_SIZE / 2,
				y: clampToChunk(input.spawnY) ?? CHUNK_SIZE / 2,
				inputX: 0,
				inputY: 0,
				sprint: false,
				disconnectedAt: null,
			};
			meta.inChunk = true;
			broadcastSnapshot(c);
		},
		leaveChunk: (c) => {
			const meta = getConnectionMeta(c, c.conn.id);
			if (!meta) return;

			meta.inChunk = false;
			const player = c.vars.players[meta.playerId];
			if (player?.connId === c.conn.id) {
				delete c.vars.players[meta.playerId];
			}
			broadcastSnapshot(c);
		},
		removePlayer: (c) => {
			const meta = getConnectionMeta(c, c.conn.id);
			if (!meta) return;

			meta.inChunk = false;
			const player = c.vars.players[meta.playerId];
			if (player?.connId === c.conn.id) {
				delete c.vars.players[meta.playerId];
			}
			broadcastSnapshot(c);
		},
		setInput: (
			c,
			input: { inputX: number; inputY: number; sprint?: boolean },
		) => {
			const player = getControlledPlayer(c, c.conn.id);
			if (!player) return;
			player.inputX = Math.max(-1, Math.min(1, input.inputX));
			player.inputY = Math.max(-1, Math.min(1, input.inputY));
			player.sprint = !!input.sprint;
		},
		// docs:end membership
		// docs:start blocks
		placeBlock: async (c, input: { gridX: number; gridY: number }) => {
			const player = getControlledPlayer(c, c.conn.id);
			if (!player) return;
			const { gridX, gridY } = input;
			if (
				gridX < 0 ||
				gridX >= GRID_COLS ||
				gridY < 0 ||
				gridY >= GRID_COLS
			)
				return;
			const key = `${gridX},${gridY}`;
			if (c.vars.blocks[key] !== player.color) {
				c.vars.blocks[key] = player.color;
				await c.db.execute(
					"INSERT OR REPLACE INTO blocks (grid_x, grid_y, color) VALUES (?, ?, ?)",
					gridX,
					gridY,
					player.color,
				);
				broadcastSnapshot(c);
			}
		},
		removeBlock: async (c, input: { gridX: number; gridY: number }) => {
			if (!getConnectionMeta(c, c.conn.id)) return;
			const key = `${input.gridX},${input.gridY}`;
			if (key in c.vars.blocks) {
				delete c.vars.blocks[key];
				await c.db.execute(
					"DELETE FROM blocks WHERE grid_x = ? AND grid_y = ?",
					input.gridX,
					input.gridY,
				);
				broadcastSnapshot(c);
			}
		},
		// docs:end blocks
		getSnapshot: (c) => buildSnapshot(c, null),
	},
});

interface Snapshot {
	worldId: string;
	chunkX: number;
	chunkY: number;
	chunkSize: number;
	tick: number;
	selfPlayerId: string | null;
	players: Record<
		string,
		{ x: number; y: number; name: string; color: string }
	>;
	blocks: Record<string, string>;
}

function buildSnapshot(c: { vars: ChunkVars }, playerId: string | null): Snapshot {
	const players: Snapshot["players"] = {};
	for (const [id, player] of Object.entries(c.vars.players)) {
		players[id] = {
			x: player.x,
			y: player.y,
			name: player.name,
			color: player.color,
		};
	}
	return {
		worldId: c.vars.worldId,
		chunkX: c.vars.chunkX,
		chunkY: c.vars.chunkY,
		chunkSize: CHUNK_SIZE,
		tick: c.vars.tick,
		selfPlayerId: playerId,
		players,
		blocks: { ...c.vars.blocks },
	};
}

// docs:start snapshot
function broadcastSnapshot(c: {
	vars: ChunkVars;
	conns: Map<
		string,
		{
			id: string;
			send: (eventName: "snapshot", data: Snapshot) => void;
		}
	>;
}) {
	for (const conn of c.conns.values()) {
		const meta = c.vars.connections[conn.id];
		const player = meta ? c.vars.players[meta.playerId] : undefined;
		const selfPlayerId =
			meta?.inChunk && player?.connId === conn.id ? meta.playerId : null;
		try {
			conn.send("snapshot", buildSnapshot(c, selfPlayerId));
		} catch {
			// Skip connections that are not fully established yet.
		}
	}
}
// docs:end snapshot

// docs:start tick
/**
 * Moves every player in the chunk each `TICK_MS` until the Actor starts
 * shutting down.
 */
async function runChunk(
	c: ActorContextOf<typeof openWorldChunk>,
): Promise<void> {
	const tick = interval(TICK_MS);
	let savedAt = Date.now();
	while (!c.aborted) {
		await tick();
		if (c.aborted) break;
		c.vars.tick += 1;

		const now = Date.now();
		for (const [id, player] of Object.entries(c.vars.players)) {
			// Drop restored players whose client did not come back.
			if (
				player.disconnectedAt !== null &&
				now - player.disconnectedAt > RECONNECT_GRACE_MS
			) {
				delete c.vars.players[id];
				continue;
			}

			const speed = player.sprint ? SPEED * SPRINT_MULTIPLIER : SPEED;
			player.x += speed * player.inputX;
			player.y += speed * player.inputY;

			player.x = Math.max(0, Math.min(CHUNK_SIZE - 1, player.x));
			player.y = Math.max(0, Math.min(CHUNK_SIZE - 1, player.y));
		}

		broadcastSnapshot(c);

		// Ticks only change memory. Save to SQLite every
		// SNAPSHOT_INTERVAL_MS instead of on every tick.
		if (now - savedAt >= SNAPSHOT_INTERVAL_MS) {
			await saveChunk(c);
			savedAt = now;
		}
	}

	// Save once more when the Actor stops, so a clean restart loses nothing.
	await saveChunk(c);
}
// docs:end tick

// docs:start save
/**
 * Writes the tick and every player position to SQLite in one transaction, so
 * a crash leaves either the previous save or this one, never part of both.
 * Blocks are not part of it, because each block action writes its own row.
 */
async function saveChunk(c: { vars: ChunkVars; db: RawAccess }) {
	const tick = c.vars.tick;
	const players: PlayerRow[] = Object.entries(c.vars.players).map(
		([playerId, player]) => ({
			player_id: playerId,
			name: player.name,
			color: player.color,
			x: player.x,
			y: player.y,
		}),
	);

	await c.db.transaction(
		async (tx) => {
			await tx.execute(
				"INSERT OR REPLACE INTO chunk (id, tick) VALUES (1, ?)",
				tick,
			);
			await tx.execute("DELETE FROM players");
			for (const player of players) {
				await tx.execute(
					"INSERT INTO players (player_id, name, color, x, y) VALUES (?, ?, ?, ?, ?)",
					player.player_id,
					player.name,
					player.color,
					player.x,
					player.y,
				);
			}
		},
		{ name: "save-chunk" },
	);
}
// docs:end save

function clampToChunk(raw: number | undefined): number | null {
	if (raw === undefined) return null;
	const value = Number(raw);
	if (!Number.isFinite(value)) return null;
	return Math.max(0, Math.min(CHUNK_SIZE - 1, value));
}

function parsePlayerId(params: unknown): string | null {
	const playerId = (params as { playerId?: string } | null)?.playerId;
	if (typeof playerId !== "string") return null;
	const trimmed = playerId.trim();
	return trimmed ? trimmed.slice(0, 64) : null;
}

function getConnectionMeta(
	c: { vars: ChunkVars },
	connId: string,
): ConnectionMeta | null {
	return c.vars.connections[connId] ?? null;
}

function getControlledPlayer(
	c: { vars: ChunkVars },
	connId: string,
): PlayerEntry | null {
	const meta = getConnectionMeta(c, connId);
	if (!meta?.inChunk) return null;
	const player = c.vars.players[meta.playerId];
	if (!player || player.connId !== connId) return null;
	return player;
}

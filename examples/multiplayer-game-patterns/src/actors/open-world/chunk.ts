import { actor, event } from "rivetkit";
import { interval } from "rivetkit/utils";
import { CHUNK_SIZE, TICK_MS, SPEED, SPRINT_MULTIPLIER } from "./config.ts";
import { getPlayerColor } from "../player-color.ts";

const GRID_COLS = Math.floor(CHUNK_SIZE / 50);

interface PlayerEntry {
	connId: string;
	name: string;
	color: string;
	x: number;
	y: number;
	inputX: number;
	inputY: number;
	sprint: boolean;
}

interface ConnectionMeta {
	playerId: string;
	inChunk: boolean;
}

interface State {
	worldId: string;
	chunkX: number;
	chunkY: number;
	tick: number;
	connections: Record<string, ConnectionMeta>;
	players: Record<string, PlayerEntry>;
	blocks: Record<string, string>;
}

// The open world is partitioned into fixed size chunks keyed by world ID and chunk coordinates.
// Clients can keep multiple chunk connections open to observe nearby state.
// Each connection carries a stable player ID while explicit enter/leave actions control chunk membership.
export const openWorldChunk = actor({
	options: { name: "Open World - Chunk", icon: "map" },
	events: {
		snapshot: event<Snapshot>(),
	},
	createState: (c): State => {
		const key = Array.isArray(c.key) ? c.key : [c.key];
		const chunkX = Number(key[1] ?? "0");
		const chunkY = Number(key[2] ?? "0");
		return {
			worldId: key[0] ?? "",
			chunkX: Number.isFinite(chunkX) ? chunkX : 0,
			chunkY: Number.isFinite(chunkY) ? chunkY : 0,
			tick: 0,
			connections: {},
			players: {},
			blocks: {},
		};
	},
	onConnect: (c, conn) => {
		if (!c.state.connections) c.state.connections = {};
		const playerId = parsePlayerId(conn.params) ?? conn.id;
		c.state.connections[conn.id] = {
			playerId,
			inChunk: false,
		};
		broadcastSnapshot(c);
	},
	onDisconnect: (c, conn) => {
		const meta = c.state.connections?.[conn.id];
		if (!meta) return;
		delete c.state.connections[conn.id];

		const player = c.state.players[meta.playerId];
		if (player?.connId === conn.id) {
			delete c.state.players[meta.playerId];
		}
		broadcastSnapshot(c);
	},
	run: async (c) => {
		if (Array.isArray(c.state.blocks)) {
			const migratedBlocks: Record<string, string> = {};
			for (const blockKey of c.state.blocks) {
				migratedBlocks[blockKey] = "#ff4f00";
			}
			c.state.blocks = migratedBlocks;
		}
		if (!c.state.blocks) c.state.blocks = {};
		if (!c.state.connections) c.state.connections = {};
		const tick = interval(TICK_MS);
		while (!c.aborted) {
			await tick();
			if (c.aborted) break;
			c.state.tick += 1;

			for (const player of Object.values(c.state.players)) {
				const speed = player.sprint ? SPEED * SPRINT_MULTIPLIER : SPEED;
				player.x += speed * player.inputX;
				player.y += speed * player.inputY;

				player.x = Math.max(0, Math.min(CHUNK_SIZE - 1, player.x));
				player.y = Math.max(0, Math.min(CHUNK_SIZE - 1, player.y));
			}

			broadcastSnapshot(c);
		}
	},
	actions: {
		enterChunk: (
			c,
			input: { name: string; spawnX?: number; spawnY?: number },
		) => {
			const meta = getConnectionMeta(c, c.conn.id);
			if (!meta) return;

			const nameRaw = input.name.trim();
			const name = nameRaw ? nameRaw.slice(0, 24) : "Player";
			const existing = c.state.players[meta.playerId];
			c.state.players[meta.playerId] = {
				connId: c.conn.id,
				name,
				color: existing?.color ?? getPlayerColor(meta.playerId),
				x: clampToChunk(input.spawnX) ?? CHUNK_SIZE / 2,
				y: clampToChunk(input.spawnY) ?? CHUNK_SIZE / 2,
				inputX: 0,
				inputY: 0,
				sprint: false,
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
			const existing = c.state.players[meta.playerId];
			c.state.players[meta.playerId] = {
				connId: c.conn.id,
				name,
				color: existing?.color ?? getPlayerColor(meta.playerId),
				x: clampToChunk(input.spawnX) ?? CHUNK_SIZE / 2,
				y: clampToChunk(input.spawnY) ?? CHUNK_SIZE / 2,
				inputX: 0,
				inputY: 0,
				sprint: false,
			};
			meta.inChunk = true;
			broadcastSnapshot(c);
		},
		leaveChunk: (c) => {
			const meta = getConnectionMeta(c, c.conn.id);
			if (!meta) return;

			meta.inChunk = false;
			const player = c.state.players[meta.playerId];
			if (player?.connId === c.conn.id) {
				delete c.state.players[meta.playerId];
			}
			broadcastSnapshot(c);
		},
		removePlayer: (c) => {
			const meta = getConnectionMeta(c, c.conn.id);
			if (!meta) return;

			meta.inChunk = false;
			const player = c.state.players[meta.playerId];
			if (player?.connId === c.conn.id) {
				delete c.state.players[meta.playerId];
			}
			broadcastSnapshot(c);
		},
		setInput: (c, input: { inputX: number; inputY: number; sprint?: boolean }) => {
			const player = getControlledPlayer(c, c.conn.id);
			if (!player) return;
			player.inputX = Math.max(-1, Math.min(1, input.inputX));
			player.inputY = Math.max(-1, Math.min(1, input.inputY));
			player.sprint = !!input.sprint;
		},
		placeBlock: (c, input: { gridX: number; gridY: number }) => {
			const player = getControlledPlayer(c, c.conn.id);
			if (!player) return;
			if (Array.isArray(c.state.blocks)) c.state.blocks = {};
			if (!c.state.blocks) c.state.blocks = {};
			const { gridX, gridY } = input;
			if (gridX < 0 || gridX >= GRID_COLS || gridY < 0 || gridY >= GRID_COLS) return;
			const key = `${gridX},${gridY}`;
			if (c.state.blocks[key] !== player.color) {
				c.state.blocks[key] = player.color;
				broadcastSnapshot(c);
			}
		},
		removeBlock: (c, input: { gridX: number; gridY: number }) => {
			if (!getConnectionMeta(c, c.conn.id)) return;
			if (Array.isArray(c.state.blocks)) c.state.blocks = {};
			if (!c.state.blocks) c.state.blocks = {};
			const key = `${input.gridX},${input.gridY}`;
			if (key in c.state.blocks) {
				delete c.state.blocks[key];
				broadcastSnapshot(c);
			}
		},
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
	players: Record<string, { x: number; y: number; name: string; color: string }>;
	blocks: Record<string, string>;
}

function buildSnapshot(
	c: { state: State },
	playerId: string | null,
): Snapshot {
	const players: Snapshot["players"] = {};
	for (const [id, player] of Object.entries(c.state.players)) {
		players[id] = { x: player.x, y: player.y, name: player.name, color: player.color };
	}
	return {
		worldId: c.state.worldId,
		chunkX: c.state.chunkX,
		chunkY: c.state.chunkY,
		chunkSize: CHUNK_SIZE,
		tick: c.state.tick,
		selfPlayerId: playerId,
		players,
		blocks: Array.isArray(c.state.blocks)
			? Object.fromEntries(c.state.blocks.map((blockKey) => [blockKey, "#ff4f00"]))
			: { ...c.state.blocks },
	};
}

function broadcastSnapshot(c: {
	state: State;
	conns: Map<
		string,
		{
			id: string;
			send: (eventName: "snapshot", data: Snapshot) => void;
		}
	>;
}) {
	for (const conn of c.conns.values()) {
		const meta = c.state.connections?.[conn.id];
		const player = meta ? c.state.players[meta.playerId] : undefined;
		const selfPlayerId =
			meta?.inChunk && player?.connId === conn.id ? meta.playerId : null;
		try {
			conn.send("snapshot", buildSnapshot(c, selfPlayerId));
		} catch {
			// Skip connections that are not fully established yet.
		}
	}
}

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
	c: { state: State },
	connId: string,
): ConnectionMeta | null {
	return c.state.connections?.[connId] ?? null;
}

function getControlledPlayer(
	c: { state: State },
	connId: string,
): PlayerEntry | null {
	const meta = getConnectionMeta(c, connId);
	if (!meta?.inChunk) return null;
	const player = c.state.players[meta.playerId];
	if (!player || player.connId !== connId) return null;
	return player;
}

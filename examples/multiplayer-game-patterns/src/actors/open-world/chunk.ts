import { actor } from "rivetkit";
import { err, sleep } from "../../utils.ts";

const DEFAULT_TICK_MS = 100;
const MIN_TICK_MS = 33;
const MAX_TICK_MS = 1000;

interface ChunkPlayer {
	playerId: string;
	name: string;
	worldX: number;
	worldY: number;
	joinedAt: number;
	updatedAt: number;
}

interface OpenWorldChunkState {
	worldId: string;
	chunkX: number;
	chunkY: number;
	chunkSize: number;
	tickMs: number;
	tick: number;
	players: Record<string, ChunkPlayer>;
	createdAt: number;
	updatedAt: number;
}

function normalizeTickMs(tickMs: number | undefined): number {
	const next = tickMs ?? DEFAULT_TICK_MS;
	if (!Number.isFinite(next) || next < MIN_TICK_MS || next > MAX_TICK_MS) {
		err(
			`tickMs must be between ${MIN_TICK_MS} and ${MAX_TICK_MS}`,
			"invalid_tick_ms",
		);
	}
	return Math.floor(next);
}

function chunkCoordFor(position: number, chunkSize: number): number {
	if (!Number.isFinite(position)) {
		err("position must be finite", "invalid_position");
	}
	return Math.floor(position / chunkSize);
}

function belongsToChunk(
	state: OpenWorldChunkState,
	input: { worldX: number; worldY: number },
) {
	const chunkX = chunkCoordFor(input.worldX, state.chunkSize);
	const chunkY = chunkCoordFor(input.worldY, state.chunkSize);
	return chunkX === state.chunkX && chunkY === state.chunkY;
}

function buildSnapshot(state: OpenWorldChunkState) {
	return {
		worldId: state.worldId,
		chunkX: state.chunkX,
		chunkY: state.chunkY,
		chunkSize: state.chunkSize,
		tickMs: state.tickMs,
		tick: state.tick,
		playerCount: Object.keys(state.players).length,
		players: state.players,
		updatedAt: state.updatedAt,
	};
}

function chunkKeyFromPosition(
	state: OpenWorldChunkState,
	input: { worldX: number; worldY: number },
): [string, string, string] {
	return [
		state.worldId,
		String(chunkCoordFor(input.worldX, state.chunkSize)),
		String(chunkCoordFor(input.worldY, state.chunkSize)),
	];
}

async function ensureDestinationChunk(
	c: { client: <T>() => any },
	input: { chunkKey: [string, string, string]; chunkSize: number; tickMs: number },
) {
	const client = c.client<any>();
	const [worldId, chunkX, chunkY] = input.chunkKey;
	try {
		await client.openWorldChunk.create(input.chunkKey, {
			input: {
				worldId,
				chunkX: Number(chunkX),
				chunkY: Number(chunkY),
				chunkSize: input.chunkSize,
				tickMs: input.tickMs,
			},
		});
	} catch {
		// The chunk may already exist.
	}
	await client.openWorldChunk.getOrCreate(input.chunkKey).ensureChunk({
		worldId,
		chunkX: Number(chunkX),
		chunkY: Number(chunkY),
		chunkSize: input.chunkSize,
		tickMs: input.tickMs,
	});
}

export const openWorldChunk = actor({
	createState: (
		c,
		input?: {
			worldId: string;
			chunkX: number;
			chunkY: number;
			chunkSize: number;
			tickMs?: number;
		},
	): OpenWorldChunkState => {
		const now = Date.now();
		const rawKey = typeof c.key[0] === "string" ? c.key[0] : "world/0/0";
		const keyParts = rawKey.split("/");
		const keyWorldId = keyParts[0] || "world";
		const keyChunkX = Number(c.key[1] ?? keyParts[1]);
		const keyChunkY = Number(c.key[2] ?? keyParts[2]);
		return {
			worldId: input?.worldId ?? keyWorldId,
			chunkX: input?.chunkX ?? (Number.isFinite(keyChunkX) ? keyChunkX : 0),
			chunkY: input?.chunkY ?? (Number.isFinite(keyChunkY) ? keyChunkY : 0),
			chunkSize: input?.chunkSize ?? 256,
			tickMs: normalizeTickMs(input?.tickMs),
			tick: 0,
			players: {},
			createdAt: now,
			updatedAt: now,
		};
	},
	connState: {
		playerId: null as string | null,
	},
	onDisconnect: (c, conn) => {
		const playerId = conn.state.playerId;
		if (!playerId) return;
		if (!c.state.players[playerId]) return;
		delete c.state.players[playerId];
		conn.state.playerId = null;
		c.state.updatedAt = Date.now();
		c.broadcast("snapshot", buildSnapshot(c.state));
		if (Object.keys(c.state.players).length === 0) {
			c.destroy();
		}
	},
	run: async (c) => {
		while (!c.aborted) {
			await sleep(c.state.tickMs);
			if (c.aborted) break;
			c.state.tick += 1;
			c.state.updatedAt = Date.now();
			c.broadcast("tick", buildSnapshot(c.state));
		}
	},
	actions: {
		ensureChunk: (
			c,
			input: {
				worldId: string;
				chunkX: number;
				chunkY: number;
				chunkSize: number;
				tickMs?: number;
			},
		) => {
			if (input.worldId !== c.state.worldId) {
				err("world does not match actor key", "world_mismatch");
			}
			if (input.chunkX !== c.state.chunkX || input.chunkY !== c.state.chunkY) {
				err("chunk coordinates do not match actor key", "chunk_mismatch");
			}
			if (
				input.chunkSize !== c.state.chunkSize &&
				Object.keys(c.state.players).length > 0
			) {
				err("chunk size does not match actor", "chunk_size_mismatch");
			}
			c.state.chunkSize = input.chunkSize;
			if (input.tickMs != null) {
				c.state.tickMs = normalizeTickMs(input.tickMs);
			}
			return buildSnapshot(c.state);
		},
		join: (c, input: { playerId: string; name: string; worldX: number; worldY: number }) => {
			if (!belongsToChunk(c.state, input)) {
				err("player position does not belong to this chunk", "wrong_chunk");
			}
			const now = Date.now();
			const existing = c.state.players[input.playerId];
			if (existing) {
				existing.worldX = input.worldX;
				existing.worldY = input.worldY;
				existing.updatedAt = now;
				c.conn.state.playerId = input.playerId;
				c.state.updatedAt = now;
				c.broadcast("snapshot", buildSnapshot(c.state));
				return { joined: true, snapshot: buildSnapshot(c.state) };
			}
			c.state.players[input.playerId] = {
				playerId: input.playerId,
				name: input.name.trim() || input.playerId,
				worldX: input.worldX,
				worldY: input.worldY,
				joinedAt: now,
				updatedAt: now,
			};
			c.conn.state.playerId = input.playerId;
			c.state.updatedAt = now;
			c.broadcast("snapshot", buildSnapshot(c.state));
			return { joined: true, snapshot: buildSnapshot(c.state) };
		},
		move: async (
			c,
			input: { playerId: string; worldX: number; worldY: number; createNextChunk?: boolean },
		) => {
			if (c.conn.state.playerId !== input.playerId) {
				err("caller does not own player", "player_mismatch");
			}
			const player = c.state.players[input.playerId];
			if (!player) {
				err("player not in this chunk", "not_joined");
			}

			const destinationKey = chunkKeyFromPosition(c.state, input);
			const isSameChunk =
				destinationKey[1] === String(c.state.chunkX) &&
				destinationKey[2] === String(c.state.chunkY);

			if (!isSameChunk) {
				if (input.createNextChunk !== false) {
					await ensureDestinationChunk(c, {
						chunkKey: destinationKey,
						chunkSize: c.state.chunkSize,
						tickMs: c.state.tickMs,
					});
				}
				delete c.state.players[input.playerId];
				c.conn.state.playerId = null;
				c.state.updatedAt = Date.now();
				c.broadcast("snapshot", buildSnapshot(c.state));
				if (Object.keys(c.state.players).length === 0) {
					c.destroy();
				}
				return {
					moved: false as const,
					reason: "cross_chunk" as const,
					nextChunkKey: destinationKey,
					snapshot: buildSnapshot(c.state),
				};
			}

			player.worldX = input.worldX;
			player.worldY = input.worldY;
			player.updatedAt = Date.now();
			c.state.updatedAt = player.updatedAt;
			c.broadcast("snapshot", buildSnapshot(c.state));
			return { moved: true as const, snapshot: buildSnapshot(c.state) };
		},
		getSnapshot: (c) => buildSnapshot(c.state),
	},
});

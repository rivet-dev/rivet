import { actor } from "rivetkit";
import { err } from "../../utils.ts";

const DEFAULT_CHUNK_SIZE = 256;
const MIN_CHUNK_SIZE = 32;
const MAX_CHUNK_SIZE = 4096;
const DEFAULT_WINDOW_RADIUS = 1;
const MAX_WINDOW_RADIUS = 4;
const DEFAULT_TICK_MS = 100;

interface WorldConfig {
	worldId: string;
	chunkSize: number;
	createdAt: number;
	updatedAt: number;
}

interface OpenWorldIndexState {
	worlds: Record<string, WorldConfig>;
}

function normalizeChunkSize(chunkSize: number | undefined): number {
	if (chunkSize == null) return DEFAULT_CHUNK_SIZE;
	if (!Number.isFinite(chunkSize)) {
		err("chunk size must be finite", "invalid_chunk_size");
	}
	const normalized = Math.floor(chunkSize);
	if (normalized < MIN_CHUNK_SIZE || normalized > MAX_CHUNK_SIZE) {
		err(
			`chunk size must be between ${MIN_CHUNK_SIZE} and ${MAX_CHUNK_SIZE}`,
			"chunk_size_out_of_range",
		);
	}
	return normalized;
}

function normalizeRadius(radius: number | undefined): number {
	const next = radius ?? DEFAULT_WINDOW_RADIUS;
	if (!Number.isInteger(next) || next < 0 || next > MAX_WINDOW_RADIUS) {
		err(
			`radius must be an integer between 0 and ${MAX_WINDOW_RADIUS}`,
			"invalid_radius",
		);
	}
	return next;
}

function chunkCoordFor(position: number, chunkSize: number): number {
	if (!Number.isFinite(position)) {
		err("position must be finite", "invalid_position");
	}
	return Math.floor(position / chunkSize);
}

function chunkActorKey(
	worldId: string,
	chunkX: number,
	chunkY: number,
): [string, string, string] {
	return [worldId, String(chunkX), String(chunkY)];
}

function ensureWorld(
	state: OpenWorldIndexState,
	input: { worldId: string; chunkSize?: number },
): WorldConfig {
	const worldId = input.worldId.trim();
	if (!worldId) {
		err("world id cannot be empty", "invalid_world_id");
	}

	const existing = state.worlds[worldId];
	if (existing) {
		if (
			input.chunkSize != null &&
			normalizeChunkSize(input.chunkSize) !== existing.chunkSize
		) {
			err(
				"cannot change chunk size for an existing world",
				"chunk_size_locked",
			);
		}
		existing.updatedAt = Date.now();
		return existing;
	}

	const now = Date.now();
	const world: WorldConfig = {
		worldId,
		chunkSize: normalizeChunkSize(input.chunkSize),
		createdAt: now,
		updatedAt: now,
	};
	state.worlds[worldId] = world;
	return world;
}

async function ensureChunkActor(
	c: { client: <T>() => any },
	input: { worldId: string; chunkX: number; chunkY: number; chunkSize: number },
) {
	const client = c.client<any>();
	const key = chunkActorKey(input.worldId, input.chunkX, input.chunkY);
	try {
		await client.openWorldChunk.create(key, {
			input: {
				worldId: input.worldId,
				chunkX: input.chunkX,
				chunkY: input.chunkY,
				chunkSize: input.chunkSize,
				tickMs: DEFAULT_TICK_MS,
			},
		});
	} catch {
		// The chunk may already exist.
	}
	await client.openWorldChunk.getOrCreate(key).ensureChunk({
		worldId: input.worldId,
		chunkX: input.chunkX,
		chunkY: input.chunkY,
		chunkSize: input.chunkSize,
		tickMs: DEFAULT_TICK_MS,
	});
}

export const openWorldIndex = actor({
	state: {
		worlds: {},
	} as OpenWorldIndexState,
	actions: {
		registerWorld: (c, input: { worldId: string; chunkSize?: number }) => {
			const world = ensureWorld(c.state, input);
			return {
				worldId: world.worldId,
				chunkSize: world.chunkSize,
				createdAt: world.createdAt,
				updatedAt: world.updatedAt,
			};
		},
		resolveChunk: async (
			c,
			input: { worldId: string; worldX: number; worldY: number; create?: boolean },
		) => {
			const world = ensureWorld(c.state, { worldId: input.worldId });
			const chunkX = chunkCoordFor(input.worldX, world.chunkSize);
			const chunkY = chunkCoordFor(input.worldY, world.chunkSize);
			if (input.create !== false) {
				await ensureChunkActor(c, {
					worldId: world.worldId,
					chunkX,
					chunkY,
					chunkSize: world.chunkSize,
				});
			}
			return {
				worldId: world.worldId,
				chunkSize: world.chunkSize,
				chunkX,
				chunkY,
				chunkKey: chunkActorKey(world.worldId, chunkX, chunkY),
			};
		},
		ensureChunk: async (
			c,
			input: { worldId: string; chunkX: number; chunkY: number; create?: boolean },
		) => {
			const world = ensureWorld(c.state, { worldId: input.worldId });
			if (input.create !== false) {
				await ensureChunkActor(c, {
					worldId: world.worldId,
					chunkX: input.chunkX,
					chunkY: input.chunkY,
					chunkSize: world.chunkSize,
				});
			}
			return {
				worldId: world.worldId,
				chunkSize: world.chunkSize,
				chunkX: input.chunkX,
				chunkY: input.chunkY,
				chunkKey: chunkActorKey(world.worldId, input.chunkX, input.chunkY),
			};
		},
		listChunkWindow: async (
			c,
			input: {
				worldId: string;
				centerWorldX: number;
				centerWorldY: number;
				radius?: number;
				create?: boolean;
			},
		) => {
			const world = ensureWorld(c.state, { worldId: input.worldId });
			const radius = normalizeRadius(input.radius);
			const centerChunkX = chunkCoordFor(input.centerWorldX, world.chunkSize);
			const centerChunkY = chunkCoordFor(input.centerWorldY, world.chunkSize);
			const chunks: Array<{
				chunkX: number;
				chunkY: number;
				chunkKey: [string, string, string];
			}> = [];

			for (let y = centerChunkY - radius; y <= centerChunkY + radius; y++) {
				for (let x = centerChunkX - radius; x <= centerChunkX + radius; x++) {
					if (input.create !== false) {
						await ensureChunkActor(c, {
							worldId: world.worldId,
							chunkX: x,
							chunkY: y,
							chunkSize: world.chunkSize,
						});
					}
					chunks.push({
						chunkX: x,
						chunkY: y,
						chunkKey: chunkActorKey(world.worldId, x, y),
					});
				}
			}

			return {
				worldId: world.worldId,
				chunkSize: world.chunkSize,
				centerChunkX,
				centerChunkY,
				radius,
				chunks,
			};
		},
	},
});

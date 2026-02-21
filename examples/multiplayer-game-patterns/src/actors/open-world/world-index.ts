import { actor, queue, UserError } from "rivetkit";

import { hasInvalidInternalToken, INTERNAL_TOKEN, isInternalToken } from "../../auth.ts";
import { registry } from "../index.ts";
import { CHUNK_SIZE, WORLD_ID } from "./config.ts";

export const openWorldIndex = actor({
	options: { name: "Open World - Index", icon: "map" },
	onBeforeConnect: (_c, params: { internalToken?: string }) => {
		if (hasInvalidInternalToken(params)) {
			throw new UserError("forbidden", { code: "forbidden" });
		}
	},
	canInvoke: (c, invoke) => {
		const isInternal = isInternalToken(
			c.conn.params as { internalToken?: string } | undefined,
		);
		if (invoke.kind === "queue" && invoke.name === "getChunkForPosition") {
			return !isInternal;
		}
		return false;
	},
	queues: {
		getChunkForPosition: queue<
			{ x: number; y: number; playerName: string },
			{ chunkKey: [string, number, number]; playerId: string; playerToken: string }
		>(),
	},
	run: async (c) => {
		for await (const message of c.queue.iter({ completable: true })) {
			try {
				if (message.name === "getChunkForPosition") {
					const { x, y, playerName } = message.body;
					const chunkX = Math.floor(x / CHUNK_SIZE);
					const chunkY = Math.floor(y / CHUNK_SIZE);

					const playerId = crypto.randomUUID();
					const playerToken = crypto.randomUUID();

					// Create player in the target chunk.
					const client = c.client<typeof registry>();
					const localX = ((x % CHUNK_SIZE) + CHUNK_SIZE) % CHUNK_SIZE;
					const localY = ((y % CHUNK_SIZE) + CHUNK_SIZE) % CHUNK_SIZE;
					const chunk = client.openWorldChunk
						.getOrCreate([WORLD_ID, String(chunkX), String(chunkY)], {
							params: { internalToken: INTERNAL_TOKEN },
						});
					await chunk.initialize({ worldId: WORLD_ID, chunkX, chunkY });
					await chunk.createPlayer({
						playerId,
						playerToken,
						playerName,
						x: localX,
						y: localY,
					});

					await message.complete({
						chunkKey: [WORLD_ID, chunkX, chunkY],
						playerId,
						playerToken,
					});
				}
			} catch (err) {
				console.error("Error processing queue message:", err);
			}
		}
	},
});

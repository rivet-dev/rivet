import type { GameClient } from "../../client.ts";
import { CHUNK_SIZE, WORLD_ID } from "../../../src/actors/open-world/config.ts";
import { OpenWorldGame } from "./open-world-game.ts";

export class OpenWorldBot {
	private game: OpenWorldGame | null = null;
	private destroyed = false;

	constructor(private client: GameClient) {
		this.start();
	}

	private async start() {
		try {
			const response = resolveChunkForPosition(300, 300);
			if (this.destroyed) return;

			this.game = new OpenWorldGame(
				null,
				this.client,
				{ ...response, playerName: `Bot-${Math.random().toString(36).slice(2, 6)}` },
				{ bot: true },
			);
		} catch {
			// Bot failed to join.
		}
	}

	destroy() {
		this.destroyed = true;
		this.game?.destroy();
	}
}

function resolveChunkForPosition(
	x: number,
	y: number,
): { chunkKey: [string, number, number]; spawnX: number; spawnY: number } {
	const chunkX = Math.floor(x / CHUNK_SIZE);
	const chunkY = Math.floor(y / CHUNK_SIZE);
	return {
		chunkKey: [WORLD_ID, chunkX, chunkY],
		spawnX: ((x % CHUNK_SIZE) + CHUNK_SIZE) % CHUNK_SIZE,
		spawnY: ((y % CHUNK_SIZE) + CHUNK_SIZE) % CHUNK_SIZE,
	};
}

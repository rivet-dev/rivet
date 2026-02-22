import type { GameClient } from "../../client.ts";
import { OpenWorldGame } from "./open-world-game.ts";

export class OpenWorldBot {
	private game: OpenWorldGame | null = null;
	private destroyed = false;

	constructor(private client: GameClient) {
		this.start();
	}

	private async start() {
		try {
			const index = this.client.openWorldIndex.getOrCreate(["main"]).connect();
			const result = await index.send(
				"getChunkForPosition",
				{ x: 300, y: 300, playerName: `Bot-${Math.random().toString(36).slice(2, 6)}` },
				{ wait: true, timeout: 10_000 },
			);
			index.dispose();
			const response = (result as { response?: { chunkKey: [string, number, number]; playerId: string; playerToken: string } })?.response;
			if (!response || this.destroyed) return;

			this.game = new OpenWorldGame(null, this.client, { ...response, playerName: "Bot" }, { bot: true });
		} catch {
			// Bot failed to join.
		}
	}

	destroy() {
		this.destroyed = true;
		this.game?.destroy();
	}
}

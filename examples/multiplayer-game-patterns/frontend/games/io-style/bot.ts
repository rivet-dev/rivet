import type { GameClient } from "../../client.ts";
import { IoGame } from "./io-game.ts";

export class IoBot {
	private game: IoGame | null = null;
	private destroyed = false;

	constructor(private client: GameClient) {
		this.start();
	}

	private async start() {
		try {
			const lobby = await this.client.ioStyleMatchmaker
				.getOrCreate(["main"])
				.findLobby();
			if (this.destroyed) return;

			this.game = new IoGame(null, this.client, lobby, { bot: true });
		} catch {
			// Bot failed to join.
		}
	}

	destroy() {
		this.destroyed = true;
		this.game?.destroy();
	}
}

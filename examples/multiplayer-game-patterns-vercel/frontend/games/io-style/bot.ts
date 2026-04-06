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
			const mm = this.client.ioStyleMatchmaker.getOrCreate(["main"]).connect();
			const result = await mm.send("findLobby", {}, { wait: true, timeout: 10_000 });
			mm.dispose();
			const response = (result as {
				response?: {
					matchId: string;
					playerId: string;
				};
			})?.response;
			if (!response || this.destroyed) return;

			this.game = new IoGame(null, this.client, response, { bot: true });
		} catch {
			// Bot failed to join.
		}
	}

	destroy() {
		this.destroyed = true;
		this.game?.destroy();
	}
}

import type { GameClient } from "../../client.ts";
import { BattleRoyaleGame } from "./battle-royale-game.ts";

export class BattleRoyaleBot {
	private game: BattleRoyaleGame | null = null;
	private destroyed = false;

	constructor(private client: GameClient) {
		this.start();
	}

	private async start() {
		try {
			const mm = this.client.battleRoyaleMatchmaker.getOrCreate(["main"]).connect();
			const result = await mm.send("findMatch", {}, { wait: true, timeout: 10_000 });
			mm.dispose();
			const response = (result as {
				response?: {
					matchId: string;
					playerId: string;
				};
			})?.response;
			if (!response || this.destroyed) return;

			this.game = new BattleRoyaleGame(null, this.client, response, { bot: true });
		} catch {
			// Bot failed to join.
		}
	}

	destroy() {
		this.destroyed = true;
		this.game?.destroy();
	}
}

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
			const match = await this.client.battleRoyaleMatchmaker
				.getOrCreate(["main"])
				.findMatch();
			if (this.destroyed) return;

			this.game = new BattleRoyaleGame(null, this.client, match, {
				bot: true,
			});
		} catch {
			// Bot failed to join.
		}
	}

	destroy() {
		this.destroyed = true;
		this.game?.destroy();
	}
}

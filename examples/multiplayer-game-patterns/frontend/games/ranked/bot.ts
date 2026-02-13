import type { GameClient } from "../../client.ts";
import type { RankedMatchInfo } from "./menu.tsx";
import { RankedGame } from "./ranked-game.ts";

export class RankedBot {
	private game: RankedGame | null = null;
	private destroyed = false;
	// biome-ignore lint/suspicious/noExplicitAny: connection handle
	private mm: any = null;

	constructor(private client: GameClient) {
		this.start();
	}

	private async start() {
		try {
			const botUsername = `Bot#${Math.floor(Math.random() * 10000).toString().padStart(4, "0")}`;
			const mm = this.client.rankedMatchmaker.getOrCreate(["main"]).connect();
			this.mm = mm;
			await mm.send("queueForMatch", { username: botUsername }, { wait: true, timeout: 120_000 });
			if (this.destroyed) return;

			// Poll for assignment until paired.
			while (!this.destroyed) {
				const assignment = await mm.getAssignment({ username: botUsername });
				if (assignment) {
					mm.dispose();
					this.mm = null;
					this.game = new RankedGame(null, this.client, assignment as RankedMatchInfo, { bot: true });
					return;
				}
				await new Promise((r) => setTimeout(r, 200));
			}
		} catch {
			// Bot failed to join.
		}
	}

	destroy() {
		this.destroyed = true;
		this.mm?.dispose();
		this.mm = null;
		this.game?.destroy();
	}
}

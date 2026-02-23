import type { GameClient } from "../../client.ts";
import { RankedGame } from "./ranked-game.ts";
import { waitForAssignment } from "./wait-for-assignment.ts";

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
			const queueResult = await mm.queueForMatch({
				username: botUsername,
			}) as { queued: boolean; connId?: string };
			if (this.destroyed) return;

			const assignment = await waitForAssignment(
				mm,
				botUsername,
				queueResult.connId,
			);
			if (this.destroyed) return;
			mm.dispose();
			this.mm = null;
			this.game = new RankedGame(null, this.client, assignment, { bot: true });
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

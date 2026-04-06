import type { GameClient } from "../../client.ts";
import type { Mode } from "../../../src/actors/arena/config.ts";
import type { ArenaMatchInfo } from "./menu.tsx";
import { ArenaGame } from "./arena-game.ts";
import { waitForAssignment } from "./wait-for-assignment.ts";

export class ArenaBot {
	private game: ArenaGame | null = null;
	private destroyed = false;
	// biome-ignore lint/suspicious/noExplicitAny: connection handle
	private mm: any = null;

	constructor(private client: GameClient, private mode: Mode) {
		this.start();
	}

	private async start() {
		try {
			const mm = this.client.arenaMatchmaker.getOrCreate(["main"]).connect();
			this.mm = mm;
			const response = await mm.queueForMatch({
				mode: this.mode,
			}) as { playerId?: string };
			if (!response?.playerId || this.destroyed) return;

			const assignment = await waitForAssignment<ArenaMatchInfo>(
				mm,
				response.playerId,
			);
			if (this.destroyed) return;
			mm.dispose();
			this.mm = null;
			this.game = new ArenaGame(null, this.client, assignment, { bot: true });
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

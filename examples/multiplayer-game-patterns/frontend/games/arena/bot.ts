import type { GameClient } from "../../client.ts";
import type { Mode } from "../../../src/actors/arena/config.ts";
import { ArenaGame } from "./arena-game.ts";

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
			const result = await mm.send("queueForMatch", { mode: this.mode }, { wait: true, timeout: 120_000 });
			const response = (result as {
				response?: { playerId: string; registrationToken: string };
			})?.response;
			if (!response || this.destroyed) return;
			await mm.registerPlayer({
				playerId: response.playerId,
				registrationToken: response.registrationToken,
			});

			// Poll for assignment until match is filled.
			while (!this.destroyed) {
				const assignment = await mm.getAssignment({
					playerId: response.playerId,
					registrationToken: response.registrationToken,
				});
				if (assignment) {
					mm.dispose();
					this.mm = null;
					this.game = new ArenaGame(null, this.client, assignment, { bot: true });
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

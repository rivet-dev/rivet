import type { PartyMatchConn } from "../../actor-types.ts";
import type { GameClient } from "../../client.ts";

export class PartyBot {
	private conn: PartyMatchConn | null = null;
	private destroyed = false;

	constructor(
		private client: GameClient,
		private partyCode: string,
	) {
		this.start();
	}

	private async start() {
		try {
			const ticket = await this.client.partyMatchmaker
				.getOrCreate(["main"])
				.joinParty({
					partyCode: this.partyCode,
					playerName: `Bot-${Math.random().toString(36).slice(2, 6)}`,
				});
			if (this.destroyed) return;

			this.conn = this.client.partyMatch
				.get([ticket.matchId], {
					params: {
						playerId: ticket.playerId,
						joinToken: ticket.joinToken,
					},
				})
				.connect();

			// Auto-ready after a short delay.
			setTimeout(() => {
				if (!this.destroyed) {
					this.conn?.toggleReady().catch(() => {});
				}
			}, 500);
		} catch {
			// Bot failed to join.
		}
	}

	destroy() {
		this.destroyed = true;
		this.conn?.dispose?.().catch(() => {});
	}
}

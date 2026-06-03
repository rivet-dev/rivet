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
			const mm = this.client.partyMatchmaker
				.getOrCreate(["main"])
				.connect();
			const result = await mm.send(
				"joinParty",
				{
					partyCode: this.partyCode,
					playerName: `Bot-${Math.random().toString(36).slice(2, 6)}`,
				},
				{ wait: true, timeout: 10_000 },
			);
			mm.dispose();
			const response = result.response;
			if (!response || this.destroyed) return;

			this.conn = this.client.partyMatch
				.get([response.matchId], {
					params: {
						playerId: response.playerId,
						joinToken: response.joinToken,
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

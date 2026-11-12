import type {
	CellValue,
	GameResult,
} from "../../../src/actors/turn-based/config.ts";
import type { TurnBasedMatchConn } from "../../actor-types.ts";
import type { GameClient } from "../../client.ts";

interface GameSnapshot {
	board: CellValue[][];
	currentTurn: "X" | "O";
	result: GameResult;
	players: Record<string, { symbol: "X" | "O" }>;
}

export class TurnBasedBot {
	private conn: TurnBasedMatchConn | null = null;
	private destroyed = false;
	private playerId = "";
	private symbol: "X" | "O" = "O";

	constructor(
		private client: GameClient,
		inviteCode: string,
	) {
		this.start(inviteCode);
	}

	private async start(inviteCode: string) {
		try {
			const game = await this.client.turnBasedMatchmaker
				.getOrCreate(["main"])
				.joinByCode({
					inviteCode,
					playerName: `Bot-${Math.random().toString(36).slice(2, 6)}`,
				});
			if (this.destroyed) return;

			this.playerId = game.playerId;

			this.conn = this.client.turnBasedMatch
				.get([game.matchId], {
					params: { playerId: game.playerId },
				})
				.connect();

			this.conn.on("gameUpdate", (snap) => {
				this.tryMove(snap);
			});

			this.conn.getSnapshot().then((snap) => {
				this.tryMove(snap as GameSnapshot);
			});
		} catch {
			// Bot failed to join.
		}
	}

	private tryMove(snap: GameSnapshot) {
		if (this.destroyed || snap.result !== null) return;
		const myPlayer = snap.players[this.playerId];
		if (!myPlayer) return;
		this.symbol = myPlayer.symbol;
		if (snap.currentTurn !== this.symbol) return;

		// Find empty cells and pick one randomly.
		const emptyCells: [number, number][] = [];
		for (let r = 0; r < snap.board.length; r++) {
			for (let c = 0; c < snap.board[r]?.length; c++) {
				if (snap.board[r]?.[c] === "") emptyCells.push([r, c]);
			}
		}
		if (emptyCells.length === 0) return;

		const [row, col] =
			emptyCells[Math.floor(Math.random() * emptyCells.length)]!;
		setTimeout(() => {
			if (!this.destroyed) {
				this.conn?.makeMove({ row, col }).catch(() => {});
			}
		}, 300);
	}

	destroy() {
		this.destroyed = true;
		this.conn?.dispose?.().catch(() => {});
	}
}

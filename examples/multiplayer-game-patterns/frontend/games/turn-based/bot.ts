import type { GameClient } from "../../client.ts";
import type { CellValue, GameResult } from "../../../src/actors/turn-based/config.ts";

interface GameSnapshot {
	board: CellValue[][];
	currentTurn: "X" | "O";
	result: GameResult;
	players: Record<string, { symbol: "X" | "O" }>;
}

export class TurnBasedBot {
	// biome-ignore lint/suspicious/noExplicitAny: connection handle
	private conn: any = null;
	private destroyed = false;
	private playerId = "";
	private symbol: "X" | "O" = "O";

	constructor(private client: GameClient, inviteCode: string) {
		this.start(inviteCode);
	}

	private async start(inviteCode: string) {
		try {
			const mm = this.client.turnBasedMatchmaker.getOrCreate(["main"]).connect();
			const result = await mm.send(
				"joinByCode",
				{ inviteCode, playerName: `Bot-${Math.random().toString(36).slice(2, 6)}` },
				{ wait: true, timeout: 10_000 },
			);
			mm.dispose();
			const response = (result as { response?: { matchId: string; playerId: string } })?.response;
			if (!response || this.destroyed) return;

			this.playerId = response.playerId;

			this.conn = this.client.turnBasedMatch
				.get([response.matchId], { params: { playerId: response.playerId } })
				.connect();

			this.conn.on("gameUpdate", (raw: unknown) => {
				this.tryMove(raw as GameSnapshot);
			});

			this.conn.getSnapshot().then((snap: unknown) => {
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
			for (let c = 0; c < snap.board[r]!.length; c++) {
				if (snap.board[r]![c] === "") emptyCells.push([r, c]);
			}
		}
		if (emptyCells.length === 0) return;

		const [row, col] = emptyCells[Math.floor(Math.random() * emptyCells.length)]!;
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

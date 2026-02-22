import { actor, type ActorContextOf, event, UserError } from "rivetkit";
import { interval } from "rivetkit/utils";
import { registry } from "../index.ts";
import { getPlayerColor } from "../player-color.ts";
import { BOARD_SIZE, type CellValue, type GameResult } from "./config.ts";

const EMPTY_MATCH_DESTROY_DELAY_MS = 10_000;

interface PlayerEntry {
	connId: string | null;
	name: string;
	color: string;
	symbol: "X" | "O";
}

interface State {
	matchId: string;
	board: CellValue[][];
	players: Record<string, PlayerEntry>;
	currentTurn: "X" | "O";
	result: GameResult;
	moveCount: number;
	emptySince: number | null;
}

export const turnBasedMatch = actor({
	options: { name: "Turn-Based - Match", icon: "chess-board" },
	events: {
		gameUpdate: event<GameSnapshot>(),
	},
	createState: (_c, input: { matchId: string }): State => {
		const board: CellValue[][] = [];
		for (let r = 0; r < BOARD_SIZE; r++) {
			board.push(Array(BOARD_SIZE).fill(""));
		}
		return {
			matchId: input.matchId,
			board,
			players: {},
			currentTurn: "X",
			result: null,
			moveCount: 0,
			emptySince: null,
		};
	},
	onConnect: (c, conn) => {
		const playerId = (conn.params as { playerId?: string })?.playerId;
		if (!playerId) return;
		const player = c.state.players[playerId];
		if (!player) {
			conn.disconnect("invalid_player");
			return;
		}
		player.connId = conn.id;
		c.state.emptySince = null;
		broadcastSnapshot(c);
	},
	onDisconnect: (c, conn) => {
		const found = findPlayerByConnId(c.state, conn.id);
		if (!found) return;
		const [, player] = found;
		player.connId = null;
		broadcastSnapshot(c);

		const anyConnected = Object.values(c.state.players).some((p) => p.connId !== null);
		if (!anyConnected) {
			c.state.emptySince = Date.now();
		}
	},
	run: async (c) => {
		const tick = interval(1_000);
		while (!c.aborted) {
			await tick();
			if (c.aborted) break;

			const anyConnected = Object.values(c.state.players).some(
				(p) => p.connId !== null,
			);
			if (anyConnected) {
				c.state.emptySince = null;
				continue;
			}
			if (c.state.emptySince === null) continue;
			if (Date.now() - c.state.emptySince >= EMPTY_MATCH_DESTROY_DELAY_MS) {
				c.destroy();
				break;
			}
		}
	},
	onDestroy: async (c) => {
		const client = c.client<typeof registry>();
		await client.turnBasedMatchmaker
			.getOrCreate(["main"])
			.send("closeMatch", { matchId: c.state.matchId });
	},
	actions: {
		createPlayer: (
			c,
			input: { playerId: string; playerName: string; symbol: "X" | "O" },
		) => {
			c.state.players[input.playerId] = {
				connId: null,
				name: input.playerName,
				color: getPlayerColor(input.playerId),
				symbol: input.symbol,
			};
		},
		makeMove: (c, input: { row: number; col: number }) => {
			if (c.state.result !== null) {
				throw new UserError("game is over", { code: "game_over" });
			}
			const found = findPlayerByConnId(c.state, c.conn.id);
			if (!found) {
				throw new UserError("player not found", { code: "player_not_found" });
			}
			const [, player] = found;
			if (player.symbol !== c.state.currentTurn) {
				throw new UserError("not your turn", { code: "not_your_turn" });
			}
			const { row, col } = input;
			if (row < 0 || row >= BOARD_SIZE || col < 0 || col >= BOARD_SIZE) {
				throw new UserError("invalid cell", { code: "invalid_cell" });
			}
			if (c.state.board[row]![col] !== "") {
				throw new UserError("cell already taken", { code: "cell_taken" });
			}

			c.state.board[row]![col] = player.symbol;
			c.state.moveCount += 1;

			const winner = checkWinner(c.state.board);
			if (winner) {
				c.state.result = winner === "X" ? "x_wins" : "o_wins";
			} else if (c.state.moveCount >= BOARD_SIZE * BOARD_SIZE) {
				c.state.result = "draw";
			} else {
				c.state.currentTurn = c.state.currentTurn === "X" ? "O" : "X";
			}

			broadcastSnapshot(c);
		},
		getSnapshot: (c) => buildSnapshot(c),
	},
});

function checkWinner(board: CellValue[][]): "X" | "O" | null {
	for (let r = 0; r < BOARD_SIZE; r++) {
		if (board[r]![0] !== "" && board[r]![0] === board[r]![1] && board[r]![1] === board[r]![2]) {
			return board[r]![0] as "X" | "O";
		}
	}
	for (let c = 0; c < BOARD_SIZE; c++) {
		if (board[0]![c] !== "" && board[0]![c] === board[1]![c] && board[1]![c] === board[2]![c]) {
			return board[0]![c] as "X" | "O";
		}
	}
	if (board[0]![0] !== "" && board[0]![0] === board[1]![1] && board[1]![1] === board[2]![2]) {
		return board[0]![0] as "X" | "O";
	}
	if (board[0]![2] !== "" && board[0]![2] === board[1]![1] && board[1]![1] === board[2]![0]) {
		return board[0]![2] as "X" | "O";
	}
	return null;
}

interface GameSnapshot {
	matchId: string;
	board: CellValue[][];
	currentTurn: "X" | "O";
	result: GameResult;
	moveCount: number;
	players: Record<string, { name: string; color: string; symbol: "X" | "O"; connected: boolean }>;
}

function buildSnapshot(c: ActorContextOf<typeof turnBasedMatch>): GameSnapshot {
	const players: GameSnapshot["players"] = {};
	for (const [id, entry] of Object.entries(c.state.players)) {
		players[id] = {
			name: entry.name,
			color: entry.color,
			symbol: entry.symbol,
			connected: entry.connId !== null,
		};
	}
	return {
		matchId: c.state.matchId,
		board: c.state.board,
		currentTurn: c.state.currentTurn,
		result: c.state.result,
		moveCount: c.state.moveCount,
		players,
	};
}

function broadcastSnapshot(c: ActorContextOf<typeof turnBasedMatch>) {
	c.broadcast("gameUpdate", buildSnapshot(c));
}

function findPlayerByConnId(
	state: State,
	connId: string,
): [string, PlayerEntry] | null {
	for (const [id, entry] of Object.entries(state.players)) {
		if (entry.connId === connId) return [id, entry];
	}
	return null;
}

import { actor, type ActorContextOf, event, UserError } from "rivetkit";
import {
	hasInvalidInternalToken,
	INTERNAL_TOKEN,
	isInternalToken,
} from "../../auth.ts";
import { registry } from "../index.ts";
import { BOARD_SIZE, type CellValue, type GameResult } from "./config.ts";

interface PlayerEntry {
	token: string;
	connId: string | null;
	name: string;
	symbol: "X" | "O";
}

interface State {
	matchId: string;
	board: CellValue[][];
	players: Record<string, PlayerEntry>;
	currentTurn: "X" | "O";
	result: GameResult;
	moveCount: number;
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
		};
	},
	onBeforeConnect: (
		c,
		params: { playerToken?: string; internalToken?: string },
	) => {
		if (hasInvalidInternalToken(params)) {
			throw new UserError("forbidden", { code: "forbidden" });
		}
		if (params?.internalToken === INTERNAL_TOKEN) return;
		const playerToken = params?.playerToken?.trim();
		if (!playerToken) {
			throw new UserError("authentication required", { code: "auth_required" });
		}
		if (!findPlayerByToken(c.state, playerToken)) {
			throw new UserError("invalid player token", { code: "invalid_player_token" });
		}
	},
	canInvoke: (c, invoke) => {
		const isInternal = isInternalToken(
			c.conn.params as { internalToken?: string } | undefined,
		);
		const isAssignedPlayer = findPlayerByConnId(c.state, c.conn.id) !== null;
		if (invoke.kind === "action" && invoke.name === "createPlayer") {
			return isInternal;
		}
		if (
			invoke.kind === "action" &&
			(invoke.name === "makeMove" || invoke.name === "getSnapshot")
		) {
			return !isInternal && isAssignedPlayer;
		}
		if (invoke.kind === "subscribe" && invoke.name === "gameUpdate") {
			return !isInternal && isAssignedPlayer;
		}
		return false;
	},
	onConnect: (c, conn) => {
		const playerToken = conn.params?.playerToken?.trim();
		if (!playerToken) return;
		const found = findPlayerByToken(c.state, playerToken);
		if (!found) {
			conn.disconnect("invalid_player_token");
			return;
		}
		const [, player] = found;
		player.connId = conn.id;
		broadcastSnapshot(c);
	},
	onDisconnect: (c, conn) => {
		const found = findPlayerByConnId(c.state, conn.id);
		if (!found) return;
		const [, player] = found;
		player.connId = null;
		broadcastSnapshot(c);

		// Destroy the match if no one is connected.
		const anyConnected = Object.values(c.state.players).some((p) => p.connId !== null);
		if (!anyConnected) {
			c.destroy();
		}
	},
	onDestroy: async (c) => {
		const client = c.client<typeof registry>();
		await client.turnBasedMatchmaker
			.getOrCreate(["main"], { params: { internalToken: INTERNAL_TOKEN } })
			.send("closeMatch", { matchId: c.state.matchId });
	},
	actions: {
		createPlayer: (
			c,
			input: { playerId: string; playerToken: string; playerName: string; symbol: "X" | "O" },
		) => {
			c.state.players[input.playerId] = {
				token: input.playerToken,
				connId: null,
				name: input.playerName,
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

			// Check win/draw.
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
	// Rows.
	for (let r = 0; r < BOARD_SIZE; r++) {
		if (board[r]![0] !== "" && board[r]![0] === board[r]![1] && board[r]![1] === board[r]![2]) {
			return board[r]![0] as "X" | "O";
		}
	}
	// Columns.
	for (let c = 0; c < BOARD_SIZE; c++) {
		if (board[0]![c] !== "" && board[0]![c] === board[1]![c] && board[1]![c] === board[2]![c]) {
			return board[0]![c] as "X" | "O";
		}
	}
	// Diagonals.
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
	players: Record<string, { name: string; symbol: "X" | "O"; connected: boolean }>;
}

function buildSnapshot(c: ActorContextOf<typeof turnBasedMatch>): GameSnapshot {
	const players: GameSnapshot["players"] = {};
	for (const [id, entry] of Object.entries(c.state.players)) {
		players[id] = {
			name: entry.name,
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

function findPlayerByToken(
	state: State,
	token: string,
): [string, PlayerEntry] | null {
	for (const [id, entry] of Object.entries(state.players)) {
		if (entry.token === token) return [id, entry];
	}
	return null;
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

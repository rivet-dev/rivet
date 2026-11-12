import { type ActorContextOf, actor, event, UserError } from "rivetkit";
import { db } from "rivetkit/db";
import type { registry } from "../index.ts";
import { getPlayerColor } from "../player-color.ts";
import { BOARD_SIZE, type CellValue, type GameResult } from "./config.ts";

const EMPTY_MATCH_DESTROY_DELAY_MS = 10_000;

type GameRow = {
	board: string;
	current_turn: "X" | "O";
	result: GameResult;
	move_count: number;
};

interface MatchVars {
	// The live connection playing each seat, keyed by player id.
	connIds: Map<string, string>;
	// When the last player left. It is null while a player is connected and
	// after a restart.
	emptySince: number | null;
}

export const turnBasedMatch = actor({
	options: { name: "Turn-Based - Match", icon: "chess-board" },
	events: {
		gameUpdate: event<GameSnapshot>(),
	},
	// docs:start state
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS game (
					id INTEGER PRIMARY KEY CHECK (id = 1),
					board TEXT NOT NULL,
					current_turn TEXT NOT NULL,
					result TEXT,
					move_count INTEGER NOT NULL
				)
			`);
			await db.execute(`
				CREATE TABLE IF NOT EXISTS players (
					player_id TEXT PRIMARY KEY,
					name TEXT NOT NULL,
					symbol TEXT NOT NULL
				)
			`);
			await db.execute(
				"INSERT OR IGNORE INTO game (id, board, current_turn, result, move_count) VALUES (1, ?, 'X', NULL, 0)",
				JSON.stringify(emptyBoard()),
			);
		},
	}),
	// Connections end when the Actor stops, so the connection playing each seat
	// only lives in memory.
	createVars: (): MatchVars => ({
		connIds: new Map(),
		emptySince: null,
	}),
	onConnect: async (c, conn) => {
		const playerId = (conn.params as { playerId?: string })?.playerId;
		if (!playerId) return;
		const [seat] = await c.db.execute(
			"SELECT player_id FROM players WHERE player_id = ?",
			playerId,
		);
		if (!seat) {
			conn.disconnect("invalid_player");
			return;
		}
		c.vars.connIds.set(playerId, conn.id);
		c.vars.emptySince = null;
		await broadcastSnapshot(c);
	},
	// docs:end state
	// docs:start abandon
	onDisconnect: async (c, conn) => {
		const playerId = findPlayerByConnId(c.vars, conn.id);
		if (!playerId) return;
		c.vars.connIds.delete(playerId);
		// A shutdown disconnects everyone, but those players come back after
		// the restart, so only a real leave starts the countdown.
		if (c.vars.connIds.size === 0 && !c.aborted) {
			// A schedule survives a sleep or restart, which a timer in memory
			// would not, and it wakes the Actor to run.
			const emptySince = Date.now();
			c.vars.emptySince = emptySince;
			await c.schedule.after(
				EMPTY_MATCH_DESTROY_DELAY_MS,
				"destroyIfEmpty",
				emptySince,
			);
		}
		await broadcastSnapshot(c);
	},
	onDestroy: async (c) => {
		const client = c.client<typeof registry>();
		await client.turnBasedMatchmaker
			.getOrCreate(["main"])
			.closeMatch({ matchId: c.key[0]! });
	},
	// docs:end abandon
	actions: {
		// docs:start abandon
		destroyIfEmpty: (c, emptySince: number) => {
			if (!c.scheduledFire) {
				throw new UserError("destroyIfEmpty only runs from a schedule", {
					code: "schedule_only",
				});
			}
			// Keep the match if a player came back. A player who came back and
			// left again armed a newer schedule, so only that one may destroy
			// the match. After a restart `emptySince` is null and nobody has
			// connected since.
			if (c.vars.connIds.size > 0) return;
			if (
				c.vars.emptySince !== null &&
				c.vars.emptySince !== emptySince
			) {
				return;
			}
			c.destroy();
		},
		// docs:end abandon
		createPlayer: async (
			c,
			input: { playerId: string; playerName: string; symbol: "X" | "O" },
		) => {
			await c.db.execute(
				"INSERT OR REPLACE INTO players (player_id, name, symbol) VALUES (?, ?, ?)",
				input.playerId,
				input.playerName,
				input.symbol,
			);
		},
		// docs:start make-move
		makeMove: async (c, input: { row: number; col: number }) => {
			const playerId = findPlayerByConnId(c.vars, c.conn.id);

			// Check and save the move in one transaction, so two moves sent at
			// once cannot both pass the turn check.
			await c.db.transaction(
				async (tx) => {
					const [game] = await tx.execute<GameRow>(
						"SELECT board, current_turn, result, move_count FROM game WHERE id = 1",
					);
					if (game!.result !== null) {
						throw new UserError("game is over", {
							code: "game_over",
						});
					}
					// A connection without a seat has a null player id and finds no row.
					const [player] = await tx.execute<{ symbol: "X" | "O" }>(
						"SELECT symbol FROM players WHERE player_id = ?",
						playerId,
					);
					if (!player) {
						throw new UserError("player not found", {
							code: "player_not_found",
						});
					}
					if (player.symbol !== game!.current_turn) {
						throw new UserError("not your turn", { code: "not_your_turn" });
					}
					const { row, col } = input;
					if (row < 0 || row >= BOARD_SIZE || col < 0 || col >= BOARD_SIZE) {
						throw new UserError("invalid cell", { code: "invalid_cell" });
					}
					const board = JSON.parse(game!.board) as CellValue[][];
					if (board[row]?.[col] !== "") {
						throw new UserError("cell already taken", {
							code: "cell_taken",
						});
					}

					board[row]![col] = player.symbol;
					const moveCount = game!.move_count + 1;
					let currentTurn = game!.current_turn;
					let result: GameResult = null;

					const winner = checkWinner(board);
					if (winner) {
						result = winner === "X" ? "x_wins" : "o_wins";
					} else if (moveCount >= BOARD_SIZE * BOARD_SIZE) {
						result = "draw";
					} else {
						currentTurn = currentTurn === "X" ? "O" : "X";
					}

					await tx.execute(
						"UPDATE game SET board = ?, current_turn = ?, result = ?, move_count = ? WHERE id = 1",
						JSON.stringify(board),
						currentTurn,
						result,
						moveCount,
					);
				},
				{ name: "make-move" },
			);

			await broadcastSnapshot(c);
		},
		// docs:end make-move
		getSnapshot: (c): Promise<GameSnapshot> => buildSnapshot(c),
	},
});

function checkWinner(board: CellValue[][]): "X" | "O" | null {
	for (let r = 0; r < BOARD_SIZE; r++) {
		if (
			board[r]?.[0] !== "" &&
			board[r]?.[0] === board[r]?.[1] &&
			board[r]?.[1] === board[r]?.[2]
		) {
			return board[r]?.[0] as "X" | "O";
		}
	}
	for (let c = 0; c < BOARD_SIZE; c++) {
		if (
			board[0]?.[c] !== "" &&
			board[0]?.[c] === board[1]?.[c] &&
			board[1]?.[c] === board[2]?.[c]
		) {
			return board[0]?.[c] as "X" | "O";
		}
	}
	if (
		board[0]?.[0] !== "" &&
		board[0]?.[0] === board[1]?.[1] &&
		board[1]?.[1] === board[2]?.[2]
	) {
		return board[0]?.[0] as "X" | "O";
	}
	if (
		board[0]?.[2] !== "" &&
		board[0]?.[2] === board[1]?.[1] &&
		board[1]?.[1] === board[2]?.[0]
	) {
		return board[0]?.[2] as "X" | "O";
	}
	return null;
}

interface GameSnapshot {
	matchId: string;
	board: CellValue[][];
	currentTurn: "X" | "O";
	result: GameResult;
	moveCount: number;
	players: Record<
		string,
		{ name: string; color: string; symbol: "X" | "O"; connected: boolean }
	>;
}

async function buildSnapshot(
	c: ActorContextOf<typeof turnBasedMatch>,
): Promise<GameSnapshot> {
	const [game] = await c.db.execute<GameRow>(
		"SELECT board, current_turn, result, move_count FROM game WHERE id = 1",
	);
	// Seats are listed in the order they were taken.
	const seats = await c.db.execute<{
		player_id: string;
		name: string;
		symbol: "X" | "O";
	}>("SELECT player_id, name, symbol FROM players ORDER BY rowid");

	const players: GameSnapshot["players"] = {};
	for (const seat of seats) {
		players[seat.player_id] = {
			name: seat.name,
			color: getPlayerColor(seat.player_id),
			symbol: seat.symbol,
			connected: c.vars.connIds.has(seat.player_id),
		};
	}
	return {
		matchId: c.key[0]!,
		board: JSON.parse(game!.board) as CellValue[][],
		currentTurn: game!.current_turn,
		result: game!.result,
		moveCount: game!.move_count,
		players,
	};
}

async function broadcastSnapshot(c: ActorContextOf<typeof turnBasedMatch>) {
	c.broadcast("gameUpdate", await buildSnapshot(c));
}

function emptyBoard(): CellValue[][] {
	const board: CellValue[][] = [];
	for (let r = 0; r < BOARD_SIZE; r++) {
		board.push(Array(BOARD_SIZE).fill(""));
	}
	return board;
}

function findPlayerByConnId(vars: MatchVars, connId: string): string | null {
	for (const [playerId, playerConnId] of vars.connIds) {
		if (playerConnId === connId) return playerId;
	}
	return null;
}

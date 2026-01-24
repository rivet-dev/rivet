import { actor, setup } from "rivetkit";

export type Player = "X" | "O";

export type GameState = {
	board: (Player | null)[];
	currentPlayer: Player;
	winner: Player | "draw" | null;
	players: { X: string | null; O: string | null };
};

const WINNING_LINES = [
	[0, 1, 2],
	[3, 4, 5],
	[6, 7, 8],
	[0, 3, 6],
	[1, 4, 7],
	[2, 5, 8],
	[0, 4, 8],
	[2, 4, 6],
];

function checkWinner(board: (Player | null)[]): Player | "draw" | null {
	for (const [a, b, c] of WINNING_LINES) {
		if (board[a] && board[a] === board[b] && board[a] === board[c]) {
			return board[a];
		}
	}
	if (board.every((cell) => cell !== null)) {
		return "draw";
	}
	return null;
}

export const ticTacToe = actor({
	// Persistent state: https://rivet.dev/docs/actors/state
	state: {
		board: Array(9).fill(null) as (Player | null)[],
		currentPlayer: "X" as Player,
		winner: null as Player | "draw" | null,
		players: { X: null, O: null } as { X: string | null; O: string | null },
	},

	actions: {
		// Join the game as X or O
		join: (c, playerId: string): { player: Player } | { error: string } => {
			if (c.state.players.X === playerId) {
				return { player: "X" };
			}
			if (c.state.players.O === playerId) {
				return { player: "O" };
			}
			if (!c.state.players.X) {
				c.state.players.X = playerId;
				c.broadcast("gameUpdate", c.state);
				return { player: "X" };
			}
			if (!c.state.players.O) {
				c.state.players.O = playerId;
				c.broadcast("gameUpdate", c.state);
				return { player: "O" };
			}
			return { error: "Game is full" };
		},

		// Make a move at position 0-8
		makeMove: (c, playerId: string, position: number) => {
			if (c.state.winner) {
				return { error: "Game is over" };
			}
			if (position < 0 || position > 8) {
				return { error: "Invalid position" };
			}
			if (c.state.board[position]) {
				return { error: "Cell already taken" };
			}

			const playerMark =
				c.state.players.X === playerId
					? "X"
					: c.state.players.O === playerId
						? "O"
						: null;
			if (!playerMark) {
				return { error: "Not a player in this game" };
			}
			if (playerMark !== c.state.currentPlayer) {
				return { error: "Not your turn" };
			}

			c.state.board[position] = playerMark;
			c.state.winner = checkWinner(c.state.board);
			if (!c.state.winner) {
				c.state.currentPlayer = c.state.currentPlayer === "X" ? "O" : "X";
			}

			c.broadcast("gameUpdate", c.state);
			return c.state;
		},

		// Get current game state
		getState: (c) => c.state,

		// Reset the game
		reset: (c) => {
			c.state.board = Array(9).fill(null);
			c.state.currentPlayer = "X";
			c.state.winner = null;
			c.broadcast("gameUpdate", c.state);
			return c.state;
		},
	},
});

// Register actors: https://rivet.dev/docs/setup
export const registry = setup({
	use: { ticTacToe },
});

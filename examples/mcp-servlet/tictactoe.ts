import {
	Actor,
	type OnBeforeConnectOptions,
	type Rpc,
	UserError,
} from "@rivet-gg/actor";
type Player = "X" | "O";

interface State {
	board: (Player | null)[][];
	winner?: Player;
}

interface ConnParams {
	player: Player;
}

interface ConnState {
	player: Player;
}

export default class TicTacToe extends Actor<State, ConnParams, ConnState> {
	#onUpdatePromise?: Promise<void>;
	#onUpdateResolve?: () => void;

	constructor() {
		super({
			rpc: {
				timeout: 60_000,
			},
		});
	}

	override _onInitialize(): State {
		// Initialize the board with null values and no winner
		return {
			board: [
				[null, null, null],
				[null, null, null],
				[null, null, null],
			],
		};
	}

	override _onBeforeConnect(
		opts: OnBeforeConnectOptions<Actor<State, ConnParams, ConnState>>,
	): ConnState {
		return { player: opts.parameters.player };
	}

	override _onStateChange() {
		if (this.#onUpdateResolve) {
			this.#onUpdateResolve();
			this.#onUpdateResolve = this.#onUpdatePromise = undefined;
		}
	}

	makeMove(rpc: Rpc<TicTacToe>, x: number, y: number): State {
		const player = rpc.connection.state.player;

		if (this._state.winner) {
			throw new UserError("Game is already over");
		}

		if (this._state.board[x][y] !== null) {
			throw new UserError("Cell is already occupied");
		}

		this._state.board[x][y] = player;

		// Check for a winner
		if (this.checkWinner(player)) {
			this._state.winner = player;
		}

		return this._state;
	}

	getBoard(rpc: Rpc<TicTacToe>): State {
		return this._state;
	}

	async waitForUpdate(rpc: Rpc<TicTacToe>): Promise<State> {
		if (!this.#onUpdateResolve) {
			const { promise, resolve } = Promise.withResolvers<void>();
			this.#onUpdatePromise = promise;
			this.#onUpdateResolve = resolve;
		}

		await this.#onUpdatePromise;

		return this._state;
	}

	private checkWinner(player: Player): boolean {
		const { board } = this._state;

		// Check rows, columns, and diagonals
		const winConditions = [
			// Rows
			[board[0][0], board[0][1], board[0][2]],
			[board[1][0], board[1][1], board[1][2]],
			[board[2][0], board[2][1], board[2][2]],
			// Columns
			[board[0][0], board[1][0], board[2][0]],
			[board[0][1], board[1][1], board[2][1]],
			[board[0][2], board[1][2], board[2][2]],
			// Diagonals
			[board[0][0], board[1][1], board[2][2]],
			[board[0][2], board[1][1], board[2][0]],
		];

		return winConditions.some((condition) =>
			condition.every((cell) => cell === player),
		);
	}
}

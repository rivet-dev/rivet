import { setupTest } from "rivetkit/test";
import { describe, expect, test } from "vitest";
import { registry } from "../src/actors.ts";

describe("tic-tac-toe", () => {
	test("players can join game", async (ctx) => {
		const { client } = await setupTest(ctx, registry);
		const game = client.ticTacToe.getOrCreate(["test-game"]);

		const player1 = await game.join("player1");
		expect(player1).toEqual({ player: "X" });

		const player2 = await game.join("player2");
		expect(player2).toEqual({ player: "O" });

		const player3 = await game.join("player3");
		expect(player3).toEqual({ error: "Game is full" });
	});

	test("existing player can rejoin", async (ctx) => {
		const { client } = await setupTest(ctx, registry);
		const game = client.ticTacToe.getOrCreate(["test-rejoin"]);

		await game.join("player1");
		await game.join("player2");

		// Same player rejoins
		const rejoin = await game.join("player1");
		expect(rejoin).toEqual({ player: "X" });
	});

	test("players take turns", async (ctx) => {
		const { client } = await setupTest(ctx, registry);
		const game = client.ticTacToe.getOrCreate(["test-turns"]);

		await game.join("player1");
		await game.join("player2");

		// X goes first
		const move1 = await game.makeMove("player1", 0);
		expect(move1).toMatchObject({ currentPlayer: "O" });

		// O's turn
		const move2 = await game.makeMove("player2", 4);
		expect(move2).toMatchObject({ currentPlayer: "X" });

		// Wrong player tries to move
		const badMove = await game.makeMove("player2", 1);
		expect(badMove).toEqual({ error: "Not your turn" });
	});

	test("cannot move to taken cell", async (ctx) => {
		const { client } = await setupTest(ctx, registry);
		const game = client.ticTacToe.getOrCreate(["test-taken"]);

		await game.join("player1");
		await game.join("player2");

		await game.makeMove("player1", 0);
		const badMove = await game.makeMove("player2", 0);
		expect(badMove).toEqual({ error: "Cell already taken" });
	});

	test("validates position range", async (ctx) => {
		const { client } = await setupTest(ctx, registry);
		const game = client.ticTacToe.getOrCreate(["test-range"]);

		await game.join("player1");

		const badMove1 = await game.makeMove("player1", -1);
		expect(badMove1).toEqual({ error: "Invalid position" });

		const badMove2 = await game.makeMove("player1", 9);
		expect(badMove2).toEqual({ error: "Invalid position" });
	});

	test("detects winner - top row", async (ctx) => {
		const { client } = await setupTest(ctx, registry);
		const game = client.ticTacToe.getOrCreate(["test-winner-row"]);

		await game.join("player1");
		await game.join("player2");

		// X wins with top row: 0, 1, 2
		await game.makeMove("player1", 0); // X
		await game.makeMove("player2", 3); // O
		await game.makeMove("player1", 1); // X
		await game.makeMove("player2", 4); // O
		const finalState = await game.makeMove("player1", 2); // X wins

		expect(finalState).toMatchObject({ winner: "X" });
	});

	test("detects winner - diagonal", async (ctx) => {
		const { client } = await setupTest(ctx, registry);
		const game = client.ticTacToe.getOrCreate(["test-winner-diag"]);

		await game.join("player1");
		await game.join("player2");

		// X wins with diagonal: 0, 4, 8
		await game.makeMove("player1", 0); // X
		await game.makeMove("player2", 1); // O
		await game.makeMove("player1", 4); // X
		await game.makeMove("player2", 2); // O
		const finalState = await game.makeMove("player1", 8); // X wins

		expect(finalState).toMatchObject({ winner: "X" });
	});

	test("detects draw", async (ctx) => {
		const { client } = await setupTest(ctx, registry);
		const game = client.ticTacToe.getOrCreate(["test-draw"]);

		await game.join("player1");
		await game.join("player2");

		// Fill board with no winner
		// X O X
		// X X O
		// O X O
		await game.makeMove("player1", 0); // X
		await game.makeMove("player2", 1); // O
		await game.makeMove("player1", 2); // X
		await game.makeMove("player2", 5); // O
		await game.makeMove("player1", 3); // X
		await game.makeMove("player2", 6); // O
		await game.makeMove("player1", 4); // X
		await game.makeMove("player2", 8); // O
		const finalState = await game.makeMove("player1", 7); // X - draw

		expect(finalState).toMatchObject({ winner: "draw" });
	});

	test("cannot move after game over", async (ctx) => {
		const { client } = await setupTest(ctx, registry);
		const game = client.ticTacToe.getOrCreate(["test-game-over"]);

		await game.join("player1");
		await game.join("player2");

		// X wins
		await game.makeMove("player1", 0);
		await game.makeMove("player2", 3);
		await game.makeMove("player1", 1);
		await game.makeMove("player2", 4);
		await game.makeMove("player1", 2); // X wins

		// Try to move after game over
		const badMove = await game.makeMove("player2", 5);
		expect(badMove).toEqual({ error: "Game is over" });
	});

	test("reset clears the board", async (ctx) => {
		const { client } = await setupTest(ctx, registry);
		const game = client.ticTacToe.getOrCreate(["test-reset"]);

		await game.join("player1");
		await game.makeMove("player1", 0);

		const resetState = await game.reset();

		expect(resetState.board).toEqual(Array(9).fill(null));
		expect(resetState.currentPlayer).toBe("X");
		expect(resetState.winner).toBe(null);
	});

	test("getState returns current state", async (ctx) => {
		const { client } = await setupTest(ctx, registry);
		const game = client.ticTacToe.getOrCreate(["test-getstate"]);

		await game.join("player1");
		await game.join("player2");
		await game.makeMove("player1", 4);

		const state = await game.getState();

		expect(state.board[4]).toBe("X");
		expect(state.currentPlayer).toBe("O");
		expect(state.players.X).toBe("player1");
		expect(state.players.O).toBe("player2");
	});
});

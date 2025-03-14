import assert from "node:assert";
import { TestClient } from "@rivet-gg/actor-client/test";
import type TicTacToe from "./tictactoe.ts";

async function main() {
	// Create two clients for different players
	const clientX = new TestClient();
	const clientO = new TestClient();

	// Get or create a TicTacToe actor instance for both players
	console.log("Creating TicTacToe actor...");
	const gameId = crypto.randomUUID();
	const gameX = await clientX.get<TicTacToe>(
		{ name: "tictactoe", game: gameId },
		{
			parameters: { player: "X" },
		},
	);
	const gameO = await clientO.get<TicTacToe>(
		{ name: "tictactoe", game: gameId },
		{
			parameters: { player: "O" },
		},
	);

	// Test initial board state
	console.log("Testing initial state...");
	const initialState = await gameX.getBoard();
	assert.ok(
		initialState.board.every((row) => row.every((cell) => cell === null)),
		"Initial board should be empty",
	);

	// Test making moves
	console.log("Testing moves...");

	// Player X moves
	const moveX = await gameX.makeMove(0, 0);
	assert.ok(moveX.board[0][0] === "X", "X's move should be recorded");

	// Player O moves
	const moveO = await gameO.makeMove(1, 1);
	assert.ok(moveO.board[1][1] === "O", "O's move should be recorded");

	// Test invalid moves
	console.log("Testing invalid moves...");
	try {
		await gameX.makeMove(0, 0);
		assert.ok(false, "Should not allow move on occupied cell");
	} catch (error) {
		assert.ok(
			error instanceof Error,
			"Should throw error for invalid move",
		);
	}

	// Test winning condition
	console.log("Testing winning condition...");
	await gameX.makeMove(0, 1);
	await gameO.makeMove(2, 2);
	await gameX.makeMove(0, 2);
	const finalState = await gameX.getBoard();
	assert.ok(finalState.winner === "X", "X should win with top row");

	// Test move after game over
	console.log("Testing moves after game over...");
	try {
		await gameO.makeMove(2, 1);
		assert.ok(false, "Should not allow moves after game is won");
	} catch (error) {
		assert.ok(
			error instanceof Error,
			"Should throw error for move after game over",
		);
	}

	console.log("All tests completed!");

	await gameX.dispose();
	await gameO.dispose();
}

main()
	.then(() => console.log("Done"))
	.catch((err) => console.error(err));

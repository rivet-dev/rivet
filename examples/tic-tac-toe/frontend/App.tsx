import { createRivetKit } from "@rivetkit/react";
import { useEffect, useState } from "react";
import type { GameState, Player, registry } from "../src/actors.ts";

const { useActor } = createRivetKit<typeof registry>(
	`${location.origin}/api/rivet`,
);

// Generate a unique player ID for this browser tab
const playerId = Math.random().toString(36).substring(2, 9);

export function App() {
	const [lobbyId, setLobbyId] = useState("default");
	const [myPlayer, setMyPlayer] = useState<Player | null>(null);
	const [isViewer, setIsViewer] = useState(false);
	const [gameState, setGameState] = useState<GameState | null>(null);

	const game = useActor({
		name: "ticTacToe",
		key: [lobbyId],
	});

	// Reset state when lobby changes
	useEffect(() => {
		setMyPlayer(null);
		setIsViewer(false);
		setGameState(null);
	}, [lobbyId]);

	// Join game and get initial state
	useEffect(() => {
		if (!game.connection) return;

		game.connection
			.join(playerId)
			.then((result: unknown) => {
				const res = result as { player?: Player; error?: string };
				if (res.player) {
					setMyPlayer(res.player);
					setIsViewer(false);
				} else if (res.error === "Game is full") {
					setIsViewer(true);
					setMyPlayer(null);
				}
			})
			.catch(() => {});

		game.connection.getState().then(setGameState).catch(() => {});
	}, [game.connection]);

	// Listen for real-time game updates
	game.useEvent("gameUpdate", (state: GameState) => {
		setGameState(state);
	});

	const handleCellClick = async (position: number) => {
		if (
			game.connection &&
			gameState &&
			!gameState.winner &&
			!gameState.board[position]
		) {
			await game.connection.makeMove(playerId, position);
		}
	};

	const handleReset = async () => {
		if (game.connection) {
			await game.connection.reset();
		}
	};

	const getStatusMessage = () => {
		if (!gameState) return "Connecting...";
		if (gameState.winner === "draw") return "It's a draw!";
		if (gameState.winner) return `Player ${gameState.winner} wins!`;
		if (isViewer) return `${gameState.currentPlayer}'s turn`;
		if (gameState.currentPlayer === myPlayer) return "Your turn";
		return `Waiting for ${gameState.currentPlayer}`;
	};

	return (
		<div className="game-container">
			<h1>Tic-Tac-Toe</h1>

			<div className="game-info">
				<div>
					You are:{" "}
					<strong className={myPlayer === "O" ? "player-o" : ""}>
						{isViewer ? "Viewer (lobby full)" : myPlayer || "..."}
					</strong>
				</div>
				<div className="status">{getStatusMessage()}</div>
			</div>

			<div className="board">
				{(gameState?.board ?? Array(9).fill(null)).map((cell, i) => (
					<button
						key={i}
						className={`cell ${cell || ""}`}
						onClick={() => handleCellClick(i)}
						disabled={
							!game.connection ||
							!gameState ||
							!!gameState.winner ||
							!!cell ||
							isViewer
						}
					/>
				))}
			</div>

			{gameState?.winner && (
				<button className="reset-btn" onClick={handleReset}>
					Play Again
				</button>
			)}

			<div className="room-controls">
				<label>Lobby ID:</label>
				<input
					type="text"
					value={lobbyId}
					onChange={(e) => setLobbyId(e.target.value)}
					placeholder="Enter lobby ID"
				/>
			</div>
		</div>
	);
}

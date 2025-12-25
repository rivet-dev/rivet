import { createRivetKit } from "@rivetkit/react";
import { useCallback, useEffect, useRef, useState } from "react";
import {
	BALL_SIZE,
	CANVAS_HEIGHT,
	CANVAS_WIDTH,
	PADDLE_HEIGHT,
	PADDLE_WIDTH,
	type GameStateEvent,
	type MatchResult,
	type PlayerSide,
} from "../shared/constants";

const { useActor } = createRivetKit("http://localhost:6420");

type GamePhase = "menu" | "searching" | "playing";

export function App() {
	const [phase, setPhase] = useState<GamePhase>("menu");
	const [matchId, setMatchId] = useState<string | null>(null);
	const [mySide, setMySide] = useState<PlayerSide | "spectator" | null>(null);
	const [gameState, setGameState] = useState<GameStateEvent | null>(null);
	const [statusMessage, setStatusMessage] = useState("");
	const canvasRef = useRef<HTMLCanvasElement>(null);

	// Connect to matchmaker (always connected for searching)
	const matchmaker = useActor({
		name: "matchmaker",
		key: ["global"],
	});

	// Connect to game room only when we have a matchId
	const pongGame = useActor({
		name: "pongGame",
		key: [matchId || "disconnected"],
		enabled: !!matchId,
	});

	// Listen for match event from matchmaker (when another player joins)
	matchmaker.useEvent("matched", (data: { matchId: string }) => {
		console.log("[frontend] received matched event", data);
		setMatchId(data.matchId);
		setPhase("playing");
		setStatusMessage("Match found! Connecting...");
	});

	// Handle matchmaker events
	const findMatch = async () => {
		if (!matchmaker.connection) return;

		setPhase("searching");
		setStatusMessage("Searching for opponent...");

		const result: MatchResult = await matchmaker.connection.findMatch();

		if (result.status === "matched" && result.matchId) {
			setMatchId(result.matchId);
			setPhase("playing");
			setStatusMessage("Match found! Connecting...");
		} else {
			setStatusMessage("Waiting for opponent...");
			// No polling needed - we'll receive a "matched" event when paired
		}
	};

	const cancelSearch = async () => {
		if (matchmaker.connection) {
			await matchmaker.connection.cancelSearch();
		}
		setPhase("menu");
		setStatusMessage("");
	};

	// Handle game connection events
	pongGame.useEvent("playerJoined", (data: { player: PlayerSide; playersConnected: number }) => {
		if (data.playersConnected === 1) {
			setStatusMessage("Waiting for second player...");
		}
	});

	pongGame.useEvent("gameStart", (state: GameStateEvent) => {
		setGameState(state);
		setStatusMessage("Game started!");
	});

	pongGame.useEvent("gameState", (state: GameStateEvent) => {
		setGameState(state);
	});

	pongGame.useEvent("playerLeft", (data: { player: PlayerSide }) => {
		setStatusMessage(`Player ${data.player} left the game`);
	});

	// Fetch initial state and player assignment when connected
	useEffect(() => {
		if (pongGame.connection) {
			pongGame.connection.getState().then((state) => {
				setGameState(state);
				if (!state.gameStarted) {
					setStatusMessage("Waiting for second player...");
				}
			});
			pongGame.connection.getPlayerAssignment().then((side: PlayerSide | "spectator" | null) => {
				if (side) {
					setMySide(side);
					if (side === "spectator") {
						setStatusMessage("Game is full - watching as spectator");
					} else {
						setStatusMessage(`You are player ${side === "left" ? "1 (Left)" : "2 (Right)"}`);
					}
				}
			});
		}
	}, [pongGame.connection]);

	// Handle keyboard input
	const handleKeyDown = useCallback(
		(e: KeyboardEvent) => {
			if (!pongGame.connection || mySide === "spectator" || !mySide) return;

			if (e.key === "ArrowUp" || e.key === "w") {
				pongGame.connection.setInput("up");
			} else if (e.key === "ArrowDown" || e.key === "s") {
				pongGame.connection.setInput("down");
			}
		},
		[pongGame.connection, mySide],
	);

	const handleKeyUp = useCallback(
		(e: KeyboardEvent) => {
			if (!pongGame.connection || mySide === "spectator" || !mySide) return;

			if (
				e.key === "ArrowUp" ||
				e.key === "w" ||
				e.key === "ArrowDown" ||
				e.key === "s"
			) {
				pongGame.connection.setInput(null);
			}
		},
		[pongGame.connection, mySide],
	);

	useEffect(() => {
		window.addEventListener("keydown", handleKeyDown);
		window.addEventListener("keyup", handleKeyUp);
		return () => {
			window.removeEventListener("keydown", handleKeyDown);
			window.removeEventListener("keyup", handleKeyUp);
		};
	}, [handleKeyDown, handleKeyUp]);

	// Render game
	useEffect(() => {
		const canvas = canvasRef.current;
		if (!canvas) return;

		const ctx = canvas.getContext("2d");
		if (!ctx) return;

		// Clear canvas
		ctx.fillStyle = "#1a1a2e";
		ctx.fillRect(0, 0, CANVAS_WIDTH, CANVAS_HEIGHT);

		// Draw center line
		ctx.strokeStyle = "#333";
		ctx.setLineDash([10, 10]);
		ctx.beginPath();
		ctx.moveTo(CANVAS_WIDTH / 2, 0);
		ctx.lineTo(CANVAS_WIDTH / 2, CANVAS_HEIGHT);
		ctx.stroke();
		ctx.setLineDash([]);

		if (!gameState) {
			// Draw waiting message
			ctx.fillStyle = "#666";
			ctx.font = "24px monospace";
			ctx.textAlign = "center";
			ctx.fillText("Waiting for players...", CANVAS_WIDTH / 2, CANVAS_HEIGHT / 2);
			return;
		}

		// Draw paddles
		ctx.fillStyle = mySide === "left" ? "#4ade80" : "#60a5fa";
		ctx.fillRect(20, gameState.leftPaddle.y, PADDLE_WIDTH, PADDLE_HEIGHT);

		ctx.fillStyle = mySide === "right" ? "#4ade80" : "#60a5fa";
		ctx.fillRect(
			CANVAS_WIDTH - 20 - PADDLE_WIDTH,
			gameState.rightPaddle.y,
			PADDLE_WIDTH,
			PADDLE_HEIGHT,
		);

		// Draw ball (only if game started)
		if (gameState.gameStarted) {
			ctx.fillStyle = "#fff";
			ctx.beginPath();
			ctx.arc(
				gameState.ball.x + BALL_SIZE / 2,
				gameState.ball.y + BALL_SIZE / 2,
				BALL_SIZE / 2,
				0,
				Math.PI * 2,
			);
			ctx.fill();
		}

		// Draw score
		ctx.fillStyle = "#fff";
		ctx.font = "48px monospace";
		ctx.textAlign = "center";
		ctx.fillText(
			`${gameState.score.left} - ${gameState.score.right}`,
			CANVAS_WIDTH / 2,
			60,
		);

		// Draw "waiting" overlay if game not started
		if (!gameState.gameStarted) {
			ctx.fillStyle = "rgba(0, 0, 0, 0.5)";
			ctx.fillRect(0, 0, CANVAS_WIDTH, CANVAS_HEIGHT);
			ctx.fillStyle = "#fff";
			ctx.font = "24px monospace";
			ctx.fillText("Waiting for opponent...", CANVAS_WIDTH / 2, CANVAS_HEIGHT / 2);
		}
	}, [gameState, mySide]);

	const resetGame = () => {
		if (pongGame.connection) {
			pongGame.connection.resetGame();
		}
	};

	const leaveGame = () => {
		setMatchId(null);
		setMySide(null);
		setGameState(null);
		setPhase("menu");
		setStatusMessage("");
	};

	// Menu phase
	if (phase === "menu") {
		return (
			<div className="game-container">
				<h1>Pong</h1>
				<p className="subtitle">Real-time multiplayer with Rivet Actors</p>

				<div className="menu">
					<button
						className="play-button"
						onClick={findMatch}
						disabled={!matchmaker.connection}
					>
						{matchmaker.connection ? "Find Match" : "Connecting..."}
					</button>
				</div>

				<div className="instructions">
					<p>Click "Find Match" to play against another player</p>
				</div>
			</div>
		);
	}

	// Searching phase
	if (phase === "searching") {
		return (
			<div className="game-container">
				<h1>Pong</h1>
				<p className="subtitle">Real-time multiplayer with Rivet Actors</p>

				<div className="searching">
					<div className="spinner"></div>
					<p>{statusMessage}</p>
					<button onClick={cancelSearch}>Cancel</button>
				</div>
			</div>
		);
	}

	// Playing phase
	return (
		<div className="game-container">
			<h1>Pong</h1>
			<p className="subtitle">Real-time multiplayer with Rivet Actors</p>

			<div className="game-info">
				<span className={`player-indicator ${mySide}`}>
					{mySide === "spectator"
						? "Spectating"
						: mySide === "left"
							? "You: Player 1 (Left)"
							: "You: Player 2 (Right)"}
				</span>
				<div className="game-buttons">
					<button onClick={resetGame} disabled={!pongGame.connection || !gameState?.gameStarted}>
						Reset
					</button>
					<button onClick={leaveGame}>Leave</button>
				</div>
			</div>

			<canvas
				ref={canvasRef}
				width={CANVAS_WIDTH}
				height={CANVAS_HEIGHT}
				className="game-canvas"
			/>

			<div className="instructions">
				{mySide !== "spectator" && (
					<p>
						Use <kbd>W</kbd>/<kbd>S</kbd> or <kbd>Arrow Up</kbd>/
						<kbd>Arrow Down</kbd> to move your paddle
					</p>
				)}
				<p className="connection-status">{statusMessage}</p>
			</div>
		</div>
	);
}

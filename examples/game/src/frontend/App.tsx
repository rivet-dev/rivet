import { createRivetKit } from "@rivetkit/react";
import { useEffect, useRef, useState } from "react";
import type { Player, registry } from "../backend/registry";

const { useActor } = createRivetKit<typeof registry>("http://localhost:6420");

export function App() {
	const [players, setPlayers] = useState<Player[]>([]);
	const [isConnected, setIsConnected] = useState(false);
	const [currentPlayerId, setCurrentPlayerId] = useState<string | null>(null);
	const canvasRef = useRef<HTMLCanvasElement>(null);
	const keysPressed = useRef<Record<string, boolean>>({});
	const inputIntervalRef = useRef<NodeJS.Timeout | null>(null);
	const animationRef = useRef<number | null>(null);

	const gameRoom = useActor({
		name: "gameRoom",
		key: ["global"],
	});

	// Track connection status
	useEffect(() => {
		setIsConnected(!!gameRoom.connection);
	}, [gameRoom.connection]);

	// Set up game controls and rendering
	useEffect(() => {
		if (!gameRoom.connection) return;

		// Set up keyboard handlers
		const handleKeyDown = (e: KeyboardEvent) => {
			keysPressed.current[e.key.toLowerCase()] = true;
		};

		const handleKeyUp = (e: KeyboardEvent) => {
			keysPressed.current[e.key.toLowerCase()] = false;
		};

		window.addEventListener("keydown", handleKeyDown);
		window.addEventListener("keyup", handleKeyUp);

		// Input update loop
		inputIntervalRef.current = setInterval(() => {
			const input = { x: 0, y: 0 };

			if (keysPressed.current["w"] || keysPressed.current["arrowup"])
				input.y = -1;
			if (keysPressed.current["s"] || keysPressed.current["arrowdown"])
				input.y = 1;
			if (keysPressed.current["a"] || keysPressed.current["arrowleft"])
				input.x = -1;
			if (keysPressed.current["d"] || keysPressed.current["arrowright"])
				input.x = 1;

			gameRoom.connection?.setInput(input);
		}, 50);

		return () => {
			window.removeEventListener("keydown", handleKeyDown);
			window.removeEventListener("keyup", handleKeyUp);
			
			if (inputIntervalRef.current) {
				clearInterval(inputIntervalRef.current);
				inputIntervalRef.current = null;
			}
		};
	}, [gameRoom.connection]);

	// Rendering loop
	useEffect(() => {
		const renderLoop = () => {
			const canvas = canvasRef.current;
			if (!canvas) return;

			const ctx = canvas.getContext("2d");
			if (!ctx) return;

			// Clear canvas
			ctx.clearRect(0, 0, canvas.width, canvas.height);

			// Draw grid
			ctx.strokeStyle = "#e0e0e0";
			ctx.lineWidth = 1;
			for (let i = 0; i <= canvas.width; i += 50) {
				ctx.beginPath();
				ctx.moveTo(i, 0);
				ctx.lineTo(i, canvas.height);
				ctx.stroke();
			}
			for (let i = 0; i <= canvas.height; i += 50) {
				ctx.beginPath();
				ctx.moveTo(0, i);
				ctx.lineTo(canvas.width, i);
				ctx.stroke();
			}

			// Draw players
			for (const player of players) {
				const isCurrentPlayer = currentPlayerId && player.id === currentPlayerId;
				
				// Draw player shadow
				ctx.fillStyle = "rgba(0, 0, 0, 0.2)";
				ctx.beginPath();
				ctx.arc(player.position.x + 2, player.position.y + 2, 12, 0, Math.PI * 2);
				ctx.fill();

				// Draw player
				ctx.fillStyle = isCurrentPlayer ? "#4287f5" : "#888";
				ctx.beginPath();
				ctx.arc(player.position.x, player.position.y, 10, 0, Math.PI * 2);
				ctx.fill();

				// Draw player border
				ctx.strokeStyle = "#333";
				ctx.lineWidth = 2;
				ctx.stroke();

				// Draw player ID
				ctx.fillStyle = "#333";
				ctx.font = "12px Arial";
				ctx.textAlign = "center";
				ctx.fillText(
					isCurrentPlayer ? "YOU" : player.id.substring(0, 8),
					player.position.x,
					player.position.y - 15
				);
			}

			animationRef.current = requestAnimationFrame(renderLoop);
		};

		animationRef.current = requestAnimationFrame(renderLoop);

		return () => {
			if (animationRef.current) {
				cancelAnimationFrame(animationRef.current);
				animationRef.current = null;
			}
		};
	}, [players, gameRoom.connection]);

	// Listen for world updates
	gameRoom.useEvent("worldUpdate", ({ playerList }: { playerList: Player[] }) => {
		setPlayers(playerList);
		
		// Try to identify current player - this is a simple approach
		// In a real implementation, we'd get the connection ID from the server
		if (currentPlayerId === null && playerList.length > 0) {
			setCurrentPlayerId(playerList[playerList.length - 1].id);
		}
	});

	return (
		<div className="app-container">
			<div className="connection-status" style={{ position: "relative" }}>
				<div className={`connection-status ${isConnected ? "connected" : "disconnected"}`}>
					{isConnected ? "Connected" : "Disconnected"}
				</div>
			</div>

			<div className="header">
				<h1>Multiplayer Game</h1>
				<p>Real-time multiplayer movement with RivetKit</p>
			</div>

			<div className="info-box">
				<h3>How to Play</h3>
				<p>
					Use WASD or arrow keys to move your character around the game world. 
					Your character is shown in blue, while other players appear in gray. 
					The game updates in real-time, so you'll see other players moving as they play.
				</p>
			</div>

			<div className="game-area">
				<canvas
					ref={canvasRef}
					width={800}
					height={600}
					className="game-canvas"
				/>
				
				<div className="player-legend">
					<div className="legend-item">
						<div className="legend-color you" />
						<span>You</span>
					</div>
					<div className="legend-item">
						<div className="legend-color other" />
						<span>Other Players</span>
					</div>
				</div>
			</div>

			<div className="controls">
				<p><strong>Controls:</strong></p>
				<p>Move: WASD or Arrow Keys</p>
				<p>Players online: {players.length}</p>
			</div>
		</div>
	);
}

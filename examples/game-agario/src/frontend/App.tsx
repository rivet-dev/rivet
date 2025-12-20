import { createRivetKit } from "@rivetkit/react";
import { useCallback, useEffect, useRef, useState } from "react";
import {
	WORLD_WIDTH,
	WORLD_HEIGHT,
	FOOD_RADIUS,
	type GameStateEvent,
	type LobbyInfo,
} from "../shared/constants";

const { useActor } = createRivetKit("http://localhost:6420");

export function App() {
	const [lobbyId, setLobbyId] = useState<string | null>(null);
	const [gameState, setGameState] = useState<GameStateEvent | null>(null);
	const [myPlayerId, setMyPlayerId] = useState<string | null>(null);
	const [viewport, setViewport] = useState({ width: window.innerWidth, height: window.innerHeight });
	const canvasRef = useRef<HTMLCanvasElement>(null);
	const mousePos = useRef({ x: window.innerWidth / 2, y: window.innerHeight / 2 });

	// Handle window resize
	useEffect(() => {
		const handleResize = () => {
			setViewport({ width: window.innerWidth, height: window.innerHeight });
		};
		window.addEventListener("resize", handleResize);
		return () => window.removeEventListener("resize", handleResize);
	}, []);

	// Connect to matchmaker
	const matchmaker = useActor({
		name: "matchmaker",
		key: ["global"],
	});

	// Get lobby assignment from matchmaker
	useEffect(() => {
		if (matchmaker.connection && !lobbyId) {
			matchmaker.connection.findLobby().then((info: LobbyInfo) => {
				setLobbyId(info.lobbyId);
			});
		}
	}, [matchmaker.connection, lobbyId]);

	// Connect to the assigned game room
	const game = useActor({
		name: "gameRoom",
		key: [lobbyId ?? ""],
		enabled: !!lobbyId,
	});


	// Listen for game state updates
	game.useEvent("gameState", (state: GameStateEvent) => {
		setGameState(state);
	});

	// Fetch initial state and player ID when connected
	useEffect(() => {
		if (game.connection) {
			game.connection.getState().then((state: GameStateEvent) => {
				setGameState(state);
			});
			game.connection.getPlayerId().then((id: string) => {
				setMyPlayerId(id);
			});
		}
	}, [game.connection]);

	// Handle mouse movement
	const handleMouseMove = useCallback(
		(e: React.MouseEvent<HTMLCanvasElement>) => {
			const canvas = canvasRef.current;
			if (!canvas || !game.connection || !gameState || !myPlayerId) return;

			const rect = canvas.getBoundingClientRect();
			const mouseX = e.clientX - rect.left;
			const mouseY = e.clientY - rect.top;

			mousePos.current = { x: mouseX, y: mouseY };

			// Find my player to calculate world coordinates
			const myPlayer = gameState.players.find((p) => p.id === myPlayerId);
			if (!myPlayer) return;

			// Convert screen coordinates to world coordinates
			const cameraX = myPlayer.x - viewport.width / 2;
			const cameraY = myPlayer.y - viewport.height / 2;

			const worldX = cameraX + mouseX;
			const worldY = cameraY + mouseY;

			game.connection.setTarget(worldX, worldY);
		},
		[game.connection, gameState, myPlayerId, viewport],
	);

	// Render game
	useEffect(() => {
		const canvas = canvasRef.current;
		if (!canvas) return;

		const ctx = canvas.getContext("2d");
		if (!ctx) return;

		const { width, height } = viewport;

		// Find my player for camera
		const myPlayer = gameState?.players.find((p) => p.id === myPlayerId);

		// Calculate camera position (centered on player)
		const cameraX = myPlayer ? myPlayer.x - width / 2 : 0;
		const cameraY = myPlayer ? myPlayer.y - height / 2 : 0;

		// Clear canvas
		ctx.fillStyle = "#1a1a2e";
		ctx.fillRect(0, 0, width, height);

		// Draw grid
		ctx.strokeStyle = "#2a2a4e";
		ctx.lineWidth = 1;
		const gridSize = 50;

		const startX = -((cameraX % gridSize) + gridSize) % gridSize;
		const startY = -((cameraY % gridSize) + gridSize) % gridSize;

		for (let x = startX; x < width; x += gridSize) {
			ctx.beginPath();
			ctx.moveTo(x, 0);
			ctx.lineTo(x, height);
			ctx.stroke();
		}

		for (let y = startY; y < height; y += gridSize) {
			ctx.beginPath();
			ctx.moveTo(0, y);
			ctx.lineTo(width, y);
			ctx.stroke();
		}

		// Draw world boundaries
		ctx.strokeStyle = "#ff4444";
		ctx.lineWidth = 3;
		ctx.strokeRect(-cameraX, -cameraY, WORLD_WIDTH, WORLD_HEIGHT);

		if (!gameState) {
			ctx.fillStyle = "#666";
			ctx.font = "24px monospace";
			ctx.textAlign = "center";
			ctx.fillText("Connecting...", width / 2, height / 2);
			return;
		}

		// Draw food
		for (const food of gameState.food) {
			const screenX = food.x - cameraX;
			const screenY = food.y - cameraY;

			// Skip if off screen
			if (
				screenX + FOOD_RADIUS < 0 ||
				screenX - FOOD_RADIUS > width ||
				screenY + FOOD_RADIUS < 0 ||
				screenY - FOOD_RADIUS > height
			) {
				continue;
			}

			ctx.beginPath();
			ctx.arc(screenX, screenY, FOOD_RADIUS, 0, Math.PI * 2);
			ctx.fillStyle = food.color;
			ctx.fill();
		}

		// Sort players by size (draw smaller ones on top)
		const sortedPlayers = [...gameState.players].sort((a, b) => b.radius - a.radius);

		// Draw players
		for (const player of sortedPlayers) {
			const screenX = player.x - cameraX;
			const screenY = player.y - cameraY;

			// Skip if off screen
			if (
				screenX + player.radius < 0 ||
				screenX - player.radius > width ||
				screenY + player.radius < 0 ||
				screenY - player.radius > height
			) {
				continue;
			}

			// Draw player circle
			ctx.beginPath();
			ctx.arc(screenX, screenY, player.radius, 0, Math.PI * 2);
			ctx.fillStyle = player.color;
			ctx.fill();

			// Draw outline for current player
			if (player.id === myPlayerId) {
				ctx.strokeStyle = "#fff";
				ctx.lineWidth = 3;
				ctx.stroke();
			} else {
				ctx.strokeStyle = "rgba(0,0,0,0.3)";
				ctx.lineWidth = 2;
				ctx.stroke();
			}

			// Draw player mass (area as score)
			const mass = Math.floor(player.radius * player.radius / 100);
			ctx.fillStyle = "#fff";
			ctx.font = `${Math.max(12, player.radius / 3)}px monospace`;
			ctx.textAlign = "center";
			ctx.textBaseline = "middle";
			ctx.fillText(String(mass), screenX, screenY);
		}

		// Draw player count
		ctx.fillStyle = "#888";
		ctx.font = "14px monospace";
		ctx.textAlign = "left";
		ctx.textBaseline = "top";
		ctx.fillText(`Players: ${gameState.players.length}`, 10, 10);

		// Draw my mass
		if (myPlayer) {
			const myMass = Math.floor(myPlayer.radius * myPlayer.radius / 100);
			ctx.fillText(`Mass: ${myMass}`, 10, 30);
		}

	}, [gameState, myPlayerId, viewport]);

	return (
		<canvas
			ref={canvasRef}
			width={viewport.width}
			height={viewport.height}
			className="game-canvas"
			onMouseMove={handleMouseMove}
		/>
	);
}

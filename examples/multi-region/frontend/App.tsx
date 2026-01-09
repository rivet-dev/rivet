import { createRivetKit } from "@rivetkit/react";
import { useEffect, useState } from "react";
import type { Player, registry } from "../src/actors.ts";

const { useActor } = createRivetKit<typeof registry>(`${window.location.origin}/api/rivet`);

export function App() {
	const [region, setRegion] = useState("us-east");
	const [players, setPlayers] = useState<Record<string, Player>>({});
	const [myPlayerId, setMyPlayerId] = useState<string | null>(null);
	const [latency, setLatency] = useState<number>(0);
	const [currentRegion, setCurrentRegion] = useState<string>("");

	// Pass region parameter to useActor - this demonstrates multi-region deployment
	const actor = useActor({
		name: "gameRoom",
		key: ["main"],
		// createInRegion isolates actor instances by region
		createInRegion: region,
		// createWithInput parameter is passed to createState
		createWithInput: { region },
	});

	// Track connection and get initial state
	useEffect(() => {
		if (!actor.connection) return;

		// Fetch initial game state
		actor.connection
			.getGameState()
			.then((state: { players: Record<string, Player>; region: string }) => {
				setPlayers(state.players);
				setCurrentRegion(state.region);
				// Set my player ID to one of the players (we'll update this when playerJoined event fires)
				const playerIds = Object.keys(state.players);
				if (playerIds.length > 0 && !myPlayerId) {
					setMyPlayerId(playerIds[playerIds.length - 1]);
				}
			})
			.catch((err: unknown) => console.error("Failed to get game state:", err));
	}, [actor.connection, myPlayerId]);

	// Handle keyboard input for movement
	useEffect(() => {
		if (!actor.connection) return;

		const handleKeyDown = (e: KeyboardEvent) => {
			let dx = 0;
			let dy = 0;

			switch (e.key) {
				case "w":
				case "ArrowUp":
					dy = -10;
					break;
				case "s":
				case "ArrowDown":
					dy = 10;
					break;
				case "a":
				case "ArrowLeft":
					dx = -10;
					break;
				case "d":
				case "ArrowRight":
					dx = 10;
					break;
			}

			if (dx !== 0 || dy !== 0) {
				const startTime = Date.now();
				actor.connection
					?.move(dx, dy)
					.then(() => {
						// Calculate round-trip latency
						setLatency(Date.now() - startTime);
					})
					.catch((err: unknown) => console.error("Move failed:", err));
			}
		};

		window.addEventListener("keydown", handleKeyDown);
		return () => window.removeEventListener("keydown", handleKeyDown);
	}, [actor.connection]);

	// Listen for player joined event
	actor.useEvent(
		"playerJoined",
		({ playerId, player }: { playerId: string; player: Player }) => {
			setPlayers((prev) => ({ ...prev, [playerId]: player }));
			// Set our player ID if we don't have one yet
			if (!myPlayerId) {
				setMyPlayerId(playerId);
			}
		},
	);

	// Listen for player left event
	actor.useEvent("playerLeft", ({ playerId }: { playerId: string }) => {
		setPlayers((prev) => {
			const newPlayers = { ...prev };
			delete newPlayers[playerId];
			return newPlayers;
		});
	});

	// Listen for player movement
	actor.useEvent(
		"playerMoved",
		({ playerId, x, y }: { playerId: string; x: number; y: number }) => {
			setPlayers((prev) => {
				const player = prev[playerId];
				if (!player) return prev;
				return {
					...prev,
					[playerId]: { ...player, x, y, lastUpdate: Date.now() },
				};
			});
		},
	);

	// Handle region change
	const handleRegionChange = (newRegion: string) => {
		setRegion(newRegion);
		setPlayers({});
		setMyPlayerId(null);
	};

	const playerCount = Object.keys(players).length;

	return (
		<div className="app-container">
			<div className="connection-status-wrapper">
				<div
					className={`connection-status ${actor.connection ? "connected" : "disconnected"}`}
				>
					{actor.connection ? "Connected" : "Disconnected"}
				</div>
			</div>

			<div className="header">
				<h1>Quickstart: Multi-Region</h1>
				<p>Multiplayer game demonstrating multi-region deployment</p>
			</div>

			<div className="info-box">
				<h3>Multi-Region Deployment</h3>
				<p>
					Select a region below to connect to a game room in that region. Each
					region has its own isolated set of actors, allowing players to connect
					to servers closer to them for lower latency.
				</p>
			</div>

			<div className="region-selector">
				<label>
					<strong>Select Region:</strong>
				</label>
				<select value={region} onChange={(e) => handleRegionChange(e.target.value)}>
					<option value="us-east">US East</option>
					<option value="eu-west">EU West</option>
					<option value="ap-south">AP South</option>
				</select>
			</div>

			<div className="region-info">
				<div className="info-card">
					<strong>Current Region:</strong> {currentRegion || region}
				</div>
				<div className="info-card">
					<strong>Players in Region:</strong> {playerCount}
				</div>
				<div className="info-card">
					<strong>Latency:</strong> {latency}ms
				</div>
			</div>

			<div className="game-area">
				<svg
					width="600"
					height="600"
					className="game-canvas"
					viewBox="0 0 1000 1000"
				>
					{/* Grid background */}
					<defs>
						<pattern
							id="grid"
							width="100"
							height="100"
							patternUnits="userSpaceOnUse"
						>
							<path
								d="M 100 0 L 0 0 0 100"
								fill="none"
								stroke="#e0e0e0"
								strokeWidth="1"
							/>
						</pattern>
					</defs>
					<rect width="1000" height="1000" fill="url(#grid)" />
					<rect
						width="1000"
						height="1000"
						fill="none"
						stroke="#333"
						strokeWidth="3"
					/>

					{/* Render all players */}
					{Object.values(players).map((player) => {
						const isMe = player.id === myPlayerId;
						return (
							<g key={player.id}>
								{/* Player shadow */}
								<circle
									cx={player.x + 2}
									cy={player.y + 2}
									r="12"
									fill="rgba(0,0,0,0.2)"
								/>
								{/* Player */}
								<circle
									cx={player.x}
									cy={player.y}
									r="10"
									fill={player.color}
									stroke="#333"
									strokeWidth="2"
								/>
								{/* Player label */}
								<text
									x={player.x}
									y={player.y - 15}
									textAnchor="middle"
									fontSize="12"
									fill="#333"
									fontWeight={isMe ? "bold" : "normal"}
								>
									{isMe ? "YOU" : player.id.substring(0, 8)}
								</text>
							</g>
						);
					})}
				</svg>
			</div>

			<div className="controls">
				<p>
					<strong>Controls:</strong>
				</p>
				<p>Move: WASD or Arrow Keys</p>
				<p>
					Each region has its own isolated game rooms. Players in different
					regions cannot see each other.
				</p>
			</div>
		</div>
	);
}

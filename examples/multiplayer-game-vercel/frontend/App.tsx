import { createRivetKit } from "@rivetkit/react";
import { useEffect, useMemo, useRef, useState } from "react";
import type { GameState, JoinResult, Player } from "../src/types.ts";
import type { registry } from "../src/actors.ts";

const { useActor } = createRivetKit<typeof registry>(`${window.location.origin}/api/rivet`);

const WORLD_SIZE = 1200;
const INPUT_RATE_MS = 50;

const getDirectionFromKeys = (keys: Set<string>) => {
	let dx = 0;
	let dy = 0;

	if (keys.has("w") || keys.has("ArrowUp")) dy -= 1;
	if (keys.has("s") || keys.has("ArrowDown")) dy += 1;
	if (keys.has("a") || keys.has("ArrowLeft")) dx -= 1;
	if (keys.has("d") || keys.has("ArrowRight")) dx += 1;

	return { dx, dy };
};

const radiusFromMass = (mass: number) => Math.max(8, Math.sqrt(mass) * 3.2);

export function App() {
	const [roomId, setRoomId] = useState<string | null>(null);
	const [players, setPlayers] = useState<Record<string, Player>>({});
	const [myPlayerId, setMyPlayerId] = useState<string | null>(null);
	const [maxPlayers, setMaxPlayers] = useState(10);
	const [displayName, setDisplayName] = useState("Pilot");
	const [status, setStatus] = useState<string>("Finding a room...");
	const [errorMessage, setErrorMessage] = useState<string | null>(null);

	const canvasRef = useRef<HTMLCanvasElement | null>(null);
	const keysRef = useRef<Set<string>>(new Set());

	const matchmaker = useActor({
		name: "matchmaker",
		key: ["main"],
	});

	const gameRoom = useActor({
		name: "gameRoom",
		key: roomId ? [roomId] : ["pending"],
		enabled: Boolean(roomId),
	});

	useEffect(() => {
		const connection = matchmaker.connection;
		if (!connection || roomId) return;

		void (async () => {
			try {
				const nextRoomId = await (await connection.findGame());
				setRoomId(nextRoomId);
				setStatus(`Connected to ${nextRoomId}`);
			} catch (err) {
				console.error("Failed to find game:", err);
				setErrorMessage("Matchmaking failed. Try again.");
			}
		})();
	}, [matchmaker.connection, roomId]);

	useEffect(() => {
		setPlayers({});
		setMyPlayerId(null);
		setErrorMessage(null);
	}, [roomId]);

	useEffect(() => {
		if (!gameRoom.connection) return;

		gameRoom.connection
			.getState()
			.then((state: GameState) => {
				setPlayers(state.players);
				setMaxPlayers(state.maxPlayers);
				setStatus(`Room ${state.roomId}`);
			})
			.catch((err: unknown) => {
				console.error("Failed to load room state:", err);
			});
	}, [gameRoom.connection]);

	gameRoom.useEvent("playerJoined", ({ playerId, player }: JoinResult) => {
		setPlayers((prev) => ({ ...prev, [playerId]: player }));
	});

	gameRoom.useEvent("playerLeft", ({ playerId }: { playerId: string }) => {
		setPlayers((prev) => {
			const next = { ...prev };
			delete next[playerId];
			return next;
		});
		if (playerId === myPlayerId) {
			setMyPlayerId(null);
		}
	});

	gameRoom.useEvent(
		"playerMoved",
		({ playerId, x, y, mass, radius }: { playerId: string; x: number; y: number; mass: number; radius: number }) => {
			setPlayers((prev) => {
				const player = prev[playerId];
				if (!player) return prev;
				return {
					...prev,
					[playerId]: {
						...player,
						x,
						y,
						mass,
						radius,
						lastUpdate: Date.now(),
					},
				};
			});
		},
	);

	gameRoom.useEvent(
		"playerEaten",
		({ eaterId, eatenId, eaterMass }: { eaterId: string; eatenId: string; eaterMass: number }) => {
			setPlayers((prev) => {
				const next = { ...prev };
				delete next[eatenId];
				if (next[eaterId]) {
					next[eaterId] = {
						...next[eaterId],
						mass: eaterMass,
						radius: radiusFromMass(eaterMass),
					};
				}
				return next;
			});

		if (eatenId === myPlayerId) {
			setMyPlayerId(null);
			setStatus("You were eaten. Rejoin to respawn.");
		}
		},
	);

	gameRoom.useEvent("gameState", (state: GameState) => {
		setPlayers(state.players);
		setMaxPlayers(state.maxPlayers);
	});

	useEffect(() => {
		const handleKeyDown = (event: KeyboardEvent) => {
			if (["ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight"].includes(event.key)) {
				event.preventDefault();
			}
			keysRef.current.add(event.key);
		};

		const handleKeyUp = (event: KeyboardEvent) => {
			keysRef.current.delete(event.key);
		};

		window.addEventListener("keydown", handleKeyDown);
		window.addEventListener("keyup", handleKeyUp);

		return () => {
			window.removeEventListener("keydown", handleKeyDown);
			window.removeEventListener("keyup", handleKeyUp);
		};
	}, []);

	useEffect(() => {
		if (!gameRoom.connection || !myPlayerId) return;

		const interval = setInterval(() => {
			const { dx, dy } = getDirectionFromKeys(keysRef.current);
			if (dx === 0 && dy === 0) return;

			gameRoom.connection
				?.move(dx, dy)
				.catch((err: unknown) => console.error("Move failed:", err));
		}, INPUT_RATE_MS);

		return () => clearInterval(interval);
	}, [gameRoom.connection, myPlayerId]);

	useEffect(() => {
		const canvas = canvasRef.current;
		if (!canvas) return;

		const ctx = canvas.getContext("2d");
		if (!ctx) return;

		ctx.clearRect(0, 0, canvas.width, canvas.height);

		const scale = canvas.width / WORLD_SIZE;

		ctx.fillStyle = "#f7f7f7";
		ctx.fillRect(0, 0, canvas.width, canvas.height);

		ctx.strokeStyle = "#e3e3e3";
		ctx.lineWidth = 1;
		for (let i = 0; i <= WORLD_SIZE; i += 100) {
			const pos = i * scale;
			ctx.beginPath();
			ctx.moveTo(pos, 0);
			ctx.lineTo(pos, canvas.height);
			ctx.stroke();

			ctx.beginPath();
			ctx.moveTo(0, pos);
			ctx.lineTo(canvas.width, pos);
			ctx.stroke();
		}

		Object.values(players).forEach((player) => {
			const isMe = player.id === myPlayerId;
			const x = player.x * scale;
			const y = player.y * scale;
			const radius = player.radius * scale;

			ctx.beginPath();
			ctx.fillStyle = player.color;
			ctx.arc(x, y, radius, 0, Math.PI * 2);
			ctx.fill();

			ctx.strokeStyle = isMe ? "#111" : "#444";
			ctx.lineWidth = isMe ? 3 : 1.5;
			ctx.stroke();

			ctx.fillStyle = "#1d1d1d";
			ctx.font = isMe ? "bold 12px Arial" : "12px Arial";
			ctx.textAlign = "center";
			ctx.fillText(isMe ? "YOU" : player.name, x, y - radius - 6);
		});
	}, [players, myPlayerId]);

	const leaderboard = useMemo(() => {
		return Object.values(players)
			.sort((a, b) => b.mass - a.mass)
			.slice(0, 5);
	}, [players]);

	const joinGame = async () => {
		if (!gameRoom.connection || !displayName.trim()) return;

		setErrorMessage(null);
		const result = await gameRoom.connection.join(displayName.trim());
		if (!result) {
			setStatus("Room full. Finding a new room...");
			const nextRoom = await matchmaker.connection?.findGame();
			if (nextRoom) {
				setRoomId(nextRoom);
				return;
			}
			setErrorMessage("No room available. Try again.");
			return;
		}

		setMyPlayerId(result.playerId);
		setPlayers((prev) => ({ ...prev, [result.playerId]: result.player }));
		setStatus("Joined the game");
	};

	const playerCount = Object.keys(players).length;

	return (
		<div className="app-shell">
			<header className="header">
				<div>
					<p className="eyebrow">Rivet multiplayer example</p>
					<h1>Multiplayer Game</h1>
					<p className="subhead">A real-time Agar.io style arena with Rivet Actors.</p>
				</div>
				<div className="status">
					<span className={gameRoom.connection ? "connected" : "disconnected"}>
						{gameRoom.connection ? "Connected" : "Disconnected"}
					</span>
					<span>{status}</span>
				</div>
			</header>

			<section className="hud">
				<div>
					<p>Room</p>
					<strong>{roomId ?? "-"}</strong>
				</div>
				<div>
					<p>Players</p>
					<strong>
						{playerCount}/{maxPlayers}
					</strong>
				</div>
				<div>
					<p>Your status</p>
					<strong>{myPlayerId ? "Alive" : "Spectating"}</strong>
				</div>
			</section>

			<section className="play-area">
				<div className="canvas-shell">
					<canvas ref={canvasRef} width={720} height={720} />
					{!myPlayerId && (
						<div className="overlay">
							<p>Enter a name to join the arena.</p>
						</div>
					)}
				</div>
				<aside className="sidebar">
					<div className="panel">
						<h3>Join the match</h3>
						<label htmlFor="player-name">Name</label>
						<input
							id="player-name"
							value={displayName}
							onChange={(event) => setDisplayName(event.target.value)}
							placeholder="Pilot name"
						/>
						<button
							onClick={joinGame}
							disabled={!gameRoom.connection || !displayName.trim()}
						>
							{myPlayerId ? "Respawn" : "Join"}
						</button>
						{errorMessage && <p className="error">{errorMessage}</p>}
					</div>
					<div className="panel">
						<h3>Leaderboard</h3>
						{leaderboard.length === 0 ? (
							<p className="muted">Waiting for players...</p>
						) : (
							<ul>
								{leaderboard.map((player) => (
									<li key={player.id} className={player.id === myPlayerId ? "me" : ""}>
										<span>{player.name}</span>
										<span>{Math.round(player.mass)}</span>
									</li>
								))}
							</ul>
						)}
					</div>
					<div className="panel">
						<h3>Controls</h3>
						<p>Move with WASD or arrow keys.</p>
						<p>Eat smaller circles to grow.</p>
						<p>Rooms auto-scale when they reach 10 players.</p>
					</div>
				</aside>
			</section>
		</div>
	);
}

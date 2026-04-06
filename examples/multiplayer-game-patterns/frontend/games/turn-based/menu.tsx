import { useEffect, useRef, useState } from "react";
import type { GameClient } from "../../client.ts";
import { waitForAssignment } from "./wait-for-assignment.ts";

export interface TurnBasedMatchInfo {
	matchId: string;
	playerId: string;
	inviteCode?: string;
}

export function TurnBasedMenu({
	client,
	onReady,
	onBack,
}: {
	client: GameClient;
	onReady: (info: TurnBasedMatchInfo) => void;
	onBack: () => void;
}) {
	const [name, setName] = useState(() => `Player#${Math.floor(Math.random() * 10000).toString().padStart(4, "0")}`);
	const [joinCode, setJoinCode] = useState("");
	const [status, setStatus] = useState<"idle" | "loading" | "waiting" | "error">("idle");
	const [error, setError] = useState("");
	// biome-ignore lint/suspicious/noExplicitAny: connection handle
	const queueConnRef = useRef<any>(null);

	const cancelQueue = () => {
		const conn = queueConnRef.current;
		queueConnRef.current = null;
		conn?.dispose();
		setStatus("idle");
	};

	useEffect(() => {
		return () => {
			const conn = queueConnRef.current;
			queueConnRef.current = null;
			conn?.dispose();
		};
	}, []);

	const findMatch = async () => {
		if (!name.trim()) return;
		setStatus("loading");
		setError("");
		try {
			const mm = client.turnBasedMatchmaker.getOrCreate(["main"]).connect();
			queueConnRef.current = mm;
			const queueResult = await mm.queueForMatch({ playerName: name.trim() }) as {
				playerId?: string;
			};
			const playerId = queueResult.playerId;
			if (!playerId) {
				throw new Error("Failed to queue for match");
			}

			setStatus("waiting");
			const assignment = await waitForAssignment<TurnBasedMatchInfo>(mm, playerId);
			if (queueConnRef.current === mm) {
				queueConnRef.current = null;
			}
			mm.dispose();
			if (!assignment?.matchId) {
				throw new Error("Timed out waiting for match");
			}
			onReady(assignment);
		} catch (err) {
			const conn = queueConnRef.current;
			queueConnRef.current = null;
			conn?.dispose();
			setError(err instanceof Error ? err.message : String(err));
			setStatus("error");
		}
	};

	const createGame = async () => {
		if (!name.trim()) return;
		setStatus("loading");
		setError("");
		try {
			const mm = client.turnBasedMatchmaker.getOrCreate(["main"]).connect();
			const result = await mm.send(
				"createGame",
				{ playerName: name.trim() },
				{ wait: true, timeout: 10_000 },
			);
			mm.dispose();
			const response = (result as { response?: TurnBasedMatchInfo })?.response;
			if (!response?.matchId) throw new Error("Failed to create game");
			onReady(response);
		} catch (err) {
			setError(err instanceof Error ? err.message : String(err));
			setStatus("error");
		}
	};

	const joinByCode = async () => {
		if (!name.trim() || !joinCode.trim()) return;
		setStatus("loading");
		setError("");
		try {
			const mm = client.turnBasedMatchmaker.getOrCreate(["main"]).connect();
			const result = await mm.send(
				"joinByCode",
				{ inviteCode: joinCode.trim(), playerName: name.trim() },
				{ wait: true, timeout: 10_000 },
			);
			mm.dispose();
			const response = (
				result as { response?: { matchId: string; playerId: string } }
			)?.response;
			if (!response?.matchId) throw new Error("Failed to join game");
			onReady(response);
		} catch (err) {
			setError(err instanceof Error ? err.message : String(err));
			setStatus("error");
		}
	};

	const isWaiting = status === "waiting";
	const isLoading = status === "loading";
	const isBusy = isWaiting || isLoading;

	return (
		<div className="app">
			<button
				className="back-link"
				onClick={() => {
					if (isWaiting) cancelQueue();
					onBack();
				}}
			>
				&larr; {isWaiting ? "Leave Queue" : "Back"}
			</button>
			<div className="menu-container">
				<h2>Turn-Based</h2>
				<p className="menu-description">
					Tic-tac-toe with invite codes and open matchmaking. Find an open match,
					create a private game, or join by code.
				</p>

				<div style={{ marginBottom: 16 }}>
					<label style={{ display: "block", color: "#8e8e93", fontSize: 12, marginBottom: 4 }}>
						Your Name
					</label>
					<input
						type="text"
						placeholder="Your name"
						value={name}
						onChange={(e) => setName(e.target.value)}
						className="text-input"
						style={{ width: "100%" }}
					/>
				</div>

				{isWaiting ? (
					<div className="queue-status">
						<div className="queue-mode-badge">Quick Play</div>
						<div className="queue-label">Waiting for match...</div>
						<button
							className="btn btn-secondary"
							onClick={cancelQueue}
							style={{ marginTop: 16 }}
						>
							Cancel Queue
						</button>
					</div>
				) : (
					<>
						<div style={{ height: 1, background: "#2c2c2e", marginBottom: 16 }} />

						<div style={{ display: "flex", gap: 16 }}>
							<div style={{ flex: 1, display: "flex", flexDirection: "column", gap: 8 }}>
								<div style={{ color: "#8e8e93", fontSize: 12, fontWeight: 600, textTransform: "uppercase", letterSpacing: 1 }}>
									Quick Play
								</div>
								<button
									className="btn btn-primary"
									onClick={() => void findMatch()}
									disabled={isBusy || !name.trim()}
									style={{ width: "100%" }}
								>
									{isLoading ? "Loading..." : "Find Match"}
								</button>
								<button
									className="btn btn-secondary"
									onClick={() => void createGame()}
									disabled={isBusy || !name.trim()}
									style={{ width: "100%" }}
								>
									{isLoading ? "Loading..." : "Create Private"}
								</button>
							</div>

							<div style={{ width: 1, background: "#2c2c2e" }} />

							<div style={{ flex: 1, display: "flex", flexDirection: "column", gap: 8 }}>
								<div style={{ color: "#8e8e93", fontSize: 12, fontWeight: 600, textTransform: "uppercase", letterSpacing: 1 }}>
									Join by Code
								</div>
								<input
									type="text"
									placeholder="Invite code"
									value={joinCode}
									onChange={(e) => setJoinCode(e.target.value.toUpperCase())}
									className="text-input"
									maxLength={6}
									style={{
										width: "100%",
										fontFamily: "ui-monospace, SFMono-Regular, 'SF Mono', Consolas, monospace",
										fontSize: 16,
										letterSpacing: 3,
										textAlign: "center",
									}}
								/>
								<button
									className="btn btn-primary"
									onClick={() => void joinByCode()}
									disabled={isBusy || !name.trim() || !joinCode.trim()}
									style={{ width: "100%" }}
								>
									{isLoading ? "Loading..." : "Join Game"}
								</button>
							</div>
						</div>
					</>
				)}

				{status === "error" && (
					<div className="status-box">
						<p className="status-error">{error}</p>
					</div>
				)}
			</div>
		</div>
	);
}

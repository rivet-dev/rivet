import { useCallback, useEffect, useRef, useState } from "react";
import type { GameClient } from "../../client.ts";
import type { LeaderboardEntry } from "../../../src/actors/ranked/leaderboard.ts";
import type { PlayerSnapshot } from "../../../src/actors/ranked/player.ts";
import { RankedBot } from "./bot.ts";

const STORAGE_KEY = "ranked_username";

function generateUsername(): string {
	return `Player#${Math.floor(Math.random() * 10000).toString().padStart(4, "0")}`;
}

function getSavedUsername(): string {
	try {
		return localStorage.getItem(STORAGE_KEY) || generateUsername();
	} catch {
		return generateUsername();
	}
}

function saveUsername(username: string) {
	try {
		localStorage.setItem(STORAGE_KEY, username);
	} catch {
		// Ignore storage errors.
	}
}

export interface RankedMatchInfo {
	matchId: string;
	username: string;
	rating: number;
	playerToken: string;
}

export function RankedMenu({
	client,
	onReady,
	onBack,
}: {
	client: GameClient;
	onReady: (info: RankedMatchInfo) => void;
	onBack: () => void;
}) {
	const [username, setUsername] = useState(getSavedUsername);
	const [status, setStatus] = useState<
		"idle" | "queuing" | "queued" | "matched" | "error"
	>("idle");
	const [queueCount, setQueueCount] = useState(0);
	const [error, setError] = useState("");
	const [profile, setProfile] = useState<PlayerSnapshot | null>(null);
	const [leaderboard, setLeaderboard] = useState<LeaderboardEntry[]>([]);

	// biome-ignore lint/suspicious/noExplicitAny: connection handle
	const mmRef = useRef<any>(null);
	// biome-ignore lint/suspicious/noExplicitAny: connection handle
	const lbRef = useRef<any>(null);
	const abortRef = useRef(false);
	const botsRef = useRef<RankedBot[]>([]);
	const matchedRef = useRef(false);

	// Load leaderboard on mount.
	useEffect(() => {
		const lb = client.rankedLeaderboard.getOrCreate(["main"]).connect();
		lbRef.current = lb;

		lb.on("leaderboardUpdate", (raw: unknown) => {
			setLeaderboard(raw as LeaderboardEntry[]);
		});

		lb.getTopScores().then((scores: unknown) => {
			setLeaderboard(scores as LeaderboardEntry[]);
		});

		return () => {
			lb.dispose();
			lbRef.current = null;
		};
	}, [client]);

	// Load profile when username changes.
	useEffect(() => {
		if (!username.trim()) {
			setProfile(null);
			return;
		}
		saveUsername(username);
		const handle = client.rankedPlayer.getOrCreate([username]);
		handle.initialize({ username })
			.then(() => handle.getProfile())
			.then((p: unknown) => setProfile(p as PlayerSnapshot))
			.catch(() => setProfile(null));
	}, [client, username]);

	const cleanup = useCallback((destroyBots = true) => {
		abortRef.current = true;
		if (destroyBots) {
			for (const bot of botsRef.current) bot.destroy();
			botsRef.current = [];
		}
		const mm = mmRef.current;
		mmRef.current = null;
		mm?.dispose();
	}, []);

	const leaveQueue = useCallback(async () => {
		await cleanup();
		setQueueCount(0);
		setStatus("idle");
	}, [cleanup]);

	const queueForMatch = async () => {
		if (!username.trim()) return;
		setStatus("queuing");
		setError("");
		abortRef.current = false;
		matchedRef.current = false;
		try {
			const mm = client.rankedMatchmaker.getOrCreate(["main"]).connect();
			mmRef.current = mm;

			let matched = false;

			const resolveMatch = (assignment: RankedMatchInfo) => {
				if (matched || abortRef.current) return;
				matched = true;
				matchedRef.current = true;
				setStatus("matched");
				setTimeout(async () => {
					await cleanup(false);
					onReady(assignment);
				}, 1500);
			};

			mm.on("queueUpdate", (raw: unknown) => {
				const data = raw as { count: number };
				setQueueCount(data.count);
			});

			setStatus("queued");

			const queueResult = await mm.send(
				"queueForMatch",
				{ username },
				{ wait: true, timeout: 120_000 },
			);
			const queueResponse = (
				queueResult as { response?: { registrationToken: string } }
			)?.response;
			if (!queueResponse || abortRef.current) {
				throw new Error("Failed to queue");
			}
			await mm.registerPlayer({
				username,
				registrationToken: queueResponse.registrationToken,
			});

			if (abortRef.current) return;

			const size = await mm.getQueueSize();
			if (!abortRef.current) setQueueCount(size as number);

			while (!matched && !abortRef.current) {
				const existing = await mm.getAssignment({
					username,
					registrationToken: queueResponse.registrationToken,
				});
				if (existing) {
					resolveMatch(existing as RankedMatchInfo);
					break;
				}
				await new Promise((resolve) => setTimeout(resolve, 200));
			}
		} catch (err) {
			if (abortRef.current) return;
			await cleanup();
			setError(err instanceof Error ? err.message : String(err));
			setStatus("error");
		}
	};

	const addBot = () => {
		const bot = new RankedBot(client);
		botsRef.current.push(bot);
	};

	useEffect(() => {
		return () => {
			cleanup(!matchedRef.current);
		};
	}, [cleanup]);

	const isQueued = status === "queued" || status === "matched";

	return (
		<div className="app">
			<button
				className="back-link"
				onClick={isQueued ? () => void leaveQueue() : onBack}
			>
				&larr; {isQueued ? "Leave Queue" : "Back"}
			</button>
			<div className="menu-container">
				<h2>Ranked</h2>
				<p className="menu-description">
					1v1 ELO-based matchmaking. Fight to be first to 5 kills. WASD to
					move, click to shoot.
				</p>

				{status === "idle" || status === "error" ? (
					<>
						<div style={{ marginBottom: 16 }}>
							<label style={{ display: "block", color: "#8e8e93", fontSize: 12, marginBottom: 4 }}>
								Username
							</label>
							<input
								type="text"
								value={username}
								onChange={(e) => setUsername(e.target.value)}
								className="text-input"
								style={{ width: "100%" }}
								placeholder="Enter username"
							/>
						</div>

						{profile && (
							<div style={{ marginBottom: 16, padding: 12, background: "#2c2c2e", borderRadius: 8 }}>
								<div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
									<span style={{ color: "#8e8e93", fontSize: 12 }}>Rating</span>
									<span style={{ color: "#ff4f00", fontSize: 20, fontWeight: 700 }}>{profile.rating}</span>
								</div>
								<div style={{ display: "flex", gap: 16, marginTop: 8 }}>
									<span style={{ color: "#30d158", fontSize: 12 }}>{profile.wins}W</span>
									<span style={{ color: "#ff3b30", fontSize: 12 }}>{profile.losses}L</span>
								</div>
							</div>
						)}

						<button
							className="btn btn-primary"
							onClick={() => void queueForMatch()}
							disabled={!username.trim()}
							style={{ width: "100%", marginBottom: 16 }}
						>
							Queue for Match
						</button>

						{status === "error" && (
							<div className="status-box">
								<p className="status-error">{error}</p>
							</div>
						)}

						{leaderboard.length > 0 && (
							<div style={{ marginTop: 8 }}>
								<div style={{ color: "#8e8e93", fontSize: 12, fontWeight: 600, textTransform: "uppercase", letterSpacing: 1, marginBottom: 8 }}>
									Leaderboard
								</div>
								<div style={{ background: "#2c2c2e", borderRadius: 8, overflow: "hidden" }}>
									{leaderboard.map((entry, i) => (
										<div
											key={entry.username}
											style={{
												display: "flex",
												justifyContent: "space-between",
												alignItems: "center",
												padding: "8px 12px",
												borderBottom: i < leaderboard.length - 1 ? "1px solid #3a3a3c" : "none",
												background: entry.username === username ? "rgba(255, 79, 0, 0.15)" : "transparent",
											}}
										>
											<div style={{ display: "flex", alignItems: "center", gap: 8 }}>
												<span style={{ color: "#6e6e73", fontSize: 12, width: 20, textAlign: "right" }}>
													{i + 1}.
												</span>
												<span style={{ color: "#ffffff", fontSize: 13 }}>
													{entry.username}
												</span>
											</div>
											<div style={{ display: "flex", alignItems: "center", gap: 12 }}>
												<span style={{ color: "#30d158", fontSize: 11 }}>{entry.wins}W</span>
												<span style={{ color: "#ff3b30", fontSize: 11 }}>{entry.losses}L</span>
												<span style={{ color: "#ff4f00", fontSize: 13, fontWeight: 600, minWidth: 40, textAlign: "right" }}>
													{entry.rating}
												</span>
											</div>
										</div>
									))}
								</div>
							</div>
						)}
					</>
				) : status === "queuing" ? (
					<div className="queue-status">
						<p className="queue-label">Joining queue...</p>
					</div>
				) : status === "queued" ? (
					<div className="queue-status">
						<div className="queue-mode-badge">
							{username} (R: {profile?.rating ?? "..."})
						</div>
						<div className="queue-count">
							{queueCount}
						</div>
						<div className="queue-label">players in queue</div>
						<div style={{ display: "flex", gap: 12, marginTop: 20 }}>
							<button
								className="btn btn-secondary"
								onClick={addBot}
							>
								Add Bot
							</button>
							<button
								className="btn btn-secondary"
								onClick={() => void leaveQueue()}
							>
								Leave Queue
							</button>
						</div>
					</div>
				) : status === "matched" ? (
					<div className="queue-status">
						<div className="match-found-text">Match Found!</div>
						<p className="queue-label">Connecting to match...</p>
					</div>
				) : null}
			</div>
		</div>
	);
}

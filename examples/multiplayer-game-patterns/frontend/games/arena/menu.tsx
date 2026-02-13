import { useCallback, useEffect, useRef, useState } from "react";
import type { GameClient } from "../../client.ts";
import { type Mode, MODE_CONFIG } from "../../../src/actors/arena/config.ts";
import type { ArenaAssignment } from "../../../src/actors/arena/matchmaker.ts";
import { ArenaBot } from "./bot.ts";

export interface ArenaMatchInfo {
	matchId: string;
	playerId: string;
	playerToken: string;
	teamId: number;
	mode: Mode;
}

const MODES: Array<{ id: Mode; label: string }> = [
	{ id: "1v1", label: "1v1" },
	{ id: "ffa", label: "FFA" },
	{ id: "duo", label: "Duo" },
	{ id: "squad", label: "Squad" },
];

export function ArenaMenu({
	client,
	onReady,
	onBack,
}: {
	client: GameClient;
	onReady: (info: ArenaMatchInfo) => void;
	onBack: () => void;
}) {
	const [mode, setMode] = useState<Mode>("ffa");
	const [status, setStatus] = useState<
		"idle" | "queuing" | "queued" | "matched" | "error"
	>("idle");
	const [queueCount, setQueueCount] = useState(0);
	const [error, setError] = useState("");

	// biome-ignore lint/suspicious/noExplicitAny: connection handle type
	const mmRef = useRef<any>(null);
	const abortRef = useRef(false);
	const botsRef = useRef<ArenaBot[]>([]);
	const matchedRef = useRef(false);

	const capacity = MODE_CONFIG[mode].capacity;

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
		setStatus("queuing");
		setError("");
		abortRef.current = false;
		matchedRef.current = false;
		try {
			const mm = client.arenaMatchmaker.getOrCreate(["main"]).connect();
			mmRef.current = mm;

			let myPlayerId: string | null = null;
			let matched = false;

			const resolveMatch = (assignment: ArenaMatchInfo) => {
				if (matched || abortRef.current) return;
				matched = true;
				matchedRef.current = true;
				setStatus("matched");
				setTimeout(async () => {
					// Don't destroy bots - they need to connect to the match.
					await cleanup(false);
					onReady(assignment);
				}, 1500);
			};

			mm.on("queueUpdate", (raw: unknown) => {
				const data = raw as { counts: Record<string, number> };
				setQueueCount(data.counts[mode] ?? 0);
			});

			mm.on("assigned", (raw: unknown) => {
				const data = raw as { assignments: ArenaAssignment[] };
				if (!myPlayerId) return;
				const mine = data.assignments.find(
					(a) => a.playerId === myPlayerId,
				);
				if (mine) resolveMatch(mine);
			});

			setStatus("queued");

			// Queue message completes immediately with playerId.
			const result = await mm.send(
				"queueForMatch",
				{ mode },
				{ wait: true, timeout: 120_000 },
			);
			const response = (
				result as { response?: { playerId: string } }
			)?.response;
			if (!response || abortRef.current)
				throw new Error("Failed to queue");
			myPlayerId = response.playerId;

			// Fetch current queue sizes since the broadcast during queue
			// processing may have fired before the WebSocket was connected.
			const sizes = await mm.getQueueSizes();
			if (!abortRef.current) setQueueCount((sizes as Record<string, number>)[mode] ?? 0);

			// Check if assignment was already made during queue processing.
			if (!matched) {
				const existing = await mm.getAssignment({
					playerId: myPlayerId,
				});
				if (existing) resolveMatch(existing as ArenaMatchInfo);
			}
		} catch (err) {
			if (abortRef.current) return;
			await cleanup();
			setError(err instanceof Error ? err.message : String(err));
			setStatus("error");
		}
	};

	const addBot = () => {
		const bot = new ArenaBot(client, mode);
		botsRef.current.push(bot);
	};

	useEffect(() => {
		return () => {
			// Don't destroy bots if we're transitioning to the game view.
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
				<h2>Arena</h2>
				<p className="menu-description">
					Mode-based fixed-capacity matches with hitscan combat. Use
					WASD to move, click to shoot. First to 10 kills wins.
				</p>

				{status === "idle" || status === "error" ? (
					<>
						<div className="mode-selector">
							{MODES.map((m) => (
								<button
									key={m.id}
									className={`btn ${mode === m.id ? "btn-primary" : "btn-secondary"}`}
									onClick={() => setMode(m.id)}
								>
									{m.label}
								</button>
							))}
						</div>
						<button
							className="btn btn-primary"
							onClick={() => void queueForMatch()}
							style={{ marginTop: 12 }}
						>
							Queue
						</button>
						{status === "error" && (
							<div className="status-box">
								<p className="status-error">{error}</p>
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
							{mode.toUpperCase()}
						</div>
						<div className="queue-count">
							{queueCount}
							<span className="queue-count-sep">/</span>
							{capacity}
						</div>
						<div className="queue-label">players in queue</div>
						<div className="queue-bar">
							<div
								className="queue-bar-fill"
								style={{
									width: `${(queueCount / capacity) * 100}%`,
								}}
							/>
						</div>
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

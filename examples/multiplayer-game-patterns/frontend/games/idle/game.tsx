import { useCallback, useEffect, useRef, useState } from "react";
import type { GameClient } from "../../client.ts";
import type { IdleMatchInfo } from "./menu.tsx";
import { BUILDINGS } from "../../../src/actors/idle/config.ts";

interface IdleSnapshot {
	playerId: string;
	playerName: string;
	resources: number;
	totalProduced: number;
	buildings: Array<{
		id: string;
		typeId: string;
		name: string;
		productionRate: number;
		productionIntervalMs: number;
		builtAt: number;
		lastCollectedAt: number;
	}>;
}

interface LeaderboardEntry {
	playerId: string;
	playerName: string;
	totalProduced: number;
}

export function IdleGame({
	client,
	matchInfo,
	onLeave,
}: {
	client: GameClient;
	matchInfo: IdleMatchInfo;
	onLeave: () => void;
}) {
	const [state, setState] = useState<IdleSnapshot | null>(null);
	const [leaderboard, setLeaderboard] = useState<LeaderboardEntry[]>([]);
	const [now, setNow] = useState(Date.now());
	// biome-ignore lint/suspicious/noExplicitAny: connection handles
	const worldRef = useRef<any>(null);
	// biome-ignore lint/suspicious/noExplicitAny: connection handles
	const lbRef = useRef<any>(null);

	const cleanup = useCallback(() => {
		const world = worldRef.current;
		const lb = lbRef.current;
		worldRef.current = null;
		lbRef.current = null;
		world?.dispose();
		lb?.dispose();
	}, []);

	useEffect(() => {
		const world = client.idleWorld
			.getOrCreate([matchInfo.playerId])
			.connect();
		worldRef.current = world;

		world.on("stateUpdate", (raw: unknown) => {
			setState(raw as IdleSnapshot);
		});

		// Initialize the actor.
		world.initialize({ playerName: matchInfo.playerName, playerId: matchInfo.playerId }).then(() => {
			return world.getState();
		}).then((s: unknown) => {
			setState(s as IdleSnapshot);
		});

		// Fetch initial leaderboard.
		world.getLeaderboard().then((lb: unknown) => {
			setLeaderboard(lb as LeaderboardEntry[]);
		});

		// Connect to leaderboard for live updates.
		const lb = client.idleLeaderboard
			.getOrCreate(["main"])
			.connect();
		lbRef.current = lb;

		lb.on("leaderboardUpdate", (raw: unknown) => {
			setLeaderboard(raw as LeaderboardEntry[]);
		});

		return () => {
			cleanup();
		};
	}, [client, matchInfo, cleanup]);

	// Timer for countdown display.
	useEffect(() => {
		const timer = setInterval(() => setNow(Date.now()), 1000);
		return () => clearInterval(timer);
	}, []);

	const buildBuilding = (typeId: string) => {
		worldRef.current?.build({ buildingTypeId: typeId }).catch(() => {});
	};

	return (
		<div className="app">
			<div className="game-header">
				<h2>Idle</h2>
				<button className="btn btn-secondary" onClick={() => { cleanup(); onLeave(); }}>
					Leave
				</button>
			</div>

			{state && (
				<div style={{ display: "grid", gridTemplateColumns: "1fr 300px", gap: 16 }}>
					<div>
						<div className="idle-resources">
							<div className="idle-resources-label">Resources</div>
							<div className="idle-resources-value">{Math.floor(state.resources)}</div>
							<div className="idle-resources-total">Total produced: {Math.floor(state.totalProduced)}</div>
						</div>

						<div className="idle-buildings-header">Buildings ({state.buildings.length})</div>
						<div className="idle-buildings-grid">
							{state.buildings.map((b) => {
								const elapsed = now - b.lastCollectedAt;
								const progress = Math.min(1, elapsed / b.productionIntervalMs);
								const remaining = Math.max(0, b.productionIntervalMs - elapsed);
								return (
									<div key={b.id} className="idle-building-card">
										<div className="idle-building-name">{b.name}</div>
										<div className="idle-building-rate">+{b.productionRate} / {b.productionIntervalMs / 1000}s</div>
										<div className="idle-building-progress">
											<div
												className="idle-building-progress-fill"
												style={{ width: `${progress * 100}%` }}
											/>
										</div>
										<div className="idle-building-timer">
											{remaining > 0 ? `${Math.ceil(remaining / 1000)}s` : "Collecting..."}
										</div>
									</div>
								);
							})}
						</div>

						<div className="idle-buildings-header" style={{ marginTop: 24 }}>Build New</div>
						<div className="idle-shop-grid">
							{BUILDINGS.map((bt) => (
								<button
									key={bt.id}
									className="idle-shop-item"
									onClick={() => buildBuilding(bt.id)}
									disabled={state.resources < bt.cost}
								>
									<div className="idle-shop-name">{bt.name}</div>
									<div className="idle-shop-cost">
										{bt.cost === 0 ? "Free" : `Cost: ${bt.cost}`}
									</div>
									<div className="idle-shop-rate">
										+{bt.productionRate} / {bt.productionIntervalMs / 1000}s
									</div>
								</button>
							))}
						</div>
					</div>

					<div>
						<div className="idle-leaderboard">
							<div className="idle-leaderboard-header">Leaderboard</div>
							{leaderboard.length === 0 ? (
								<div className="empty-state" style={{ padding: 16 }}>No scores yet</div>
							) : (
								leaderboard.map((entry, i) => (
									<div
										key={entry.playerId}
										className={`idle-leaderboard-row ${entry.playerId === matchInfo.playerId ? "idle-leaderboard-me" : ""}`}
									>
										<span className="idle-leaderboard-rank">#{i + 1}</span>
										<span className="idle-leaderboard-name">{entry.playerName}</span>
										<span className="idle-leaderboard-score">{entry.totalProduced}</span>
									</div>
								))
							)}
						</div>
					</div>
				</div>
			)}
		</div>
	);
}

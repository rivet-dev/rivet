import { useState } from "react";
import type { GameClient } from "../../client.ts";

export interface IdleMatchInfo {
	playerId: string;
	playerName: string;
}

export function IdleMenu({
	client,
	onReady,
	onBack,
}: {
	client: GameClient;
	onReady: (info: IdleMatchInfo) => void;
	onBack: () => void;
}) {
	// Suppress unused variable warning; client is needed by interface.
	void client;
	const [name, setName] = useState(() => `Player#${Math.floor(Math.random() * 10000).toString().padStart(4, "0")}`);

	const startPlaying = () => {
		if (!name.trim()) return;
		const playerId = crypto.randomUUID();
		onReady({ playerId, playerName: name.trim() });
	};

	return (
		<div className="app">
			<button className="back-link" onClick={onBack}>
				&larr; Back
			</button>
			<div className="menu-container">
				<h2>Idle</h2>
				<p className="menu-description">
					Build production buildings that generate resources even while offline.
					Uses scheduled actions for offline progression and a global
					leaderboard.
				</p>
				<div style={{ marginBottom: 16 }}>
					<input
						type="text"
						placeholder="Your name"
						value={name}
						onChange={(e) => setName(e.target.value)}
						className="text-input"
						style={{ width: "100%" }}
					/>
				</div>
				<button
					className="btn btn-primary"
					onClick={startPlaying}
					disabled={!name.trim()}
				>
					Start Playing
				</button>
			</div>
		</div>
	);
}

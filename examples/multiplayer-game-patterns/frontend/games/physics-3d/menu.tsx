import { useState } from "react";
import type { GameClient } from "../../client.ts";

export interface Physics3dMatchInfo {
	name: string;
}

function generatePlayerName(): string {
	return `Player#${Math.floor(Math.random() * 10000).toString().padStart(4, "0")}`;
}

export function Physics3dMenu({
	onReady,
	onBack,
}: {
	client: GameClient;
	onReady: (info: Physics3dMatchInfo) => void;
	onBack: () => void;
}) {
	const [name, setName] = useState(generatePlayerName);

	return (
		<div className="app">
			<button className="back-link" onClick={onBack}>
				&larr; Back
			</button>
			<div className="menu-container">
				<h2>Physics 3D</h2>
				<p className="menu-description">
					Shared Rapier 3D physics at 10 TPS with client-side prediction and
					network smoothing. WASD to move, Space to jump. Click to spawn cubes.
				</p>
				<div style={{ marginBottom: 16 }}>
					<input
						type="text"
						className="text-input"
						value={name}
						onChange={(e) => setName(e.target.value)}
						placeholder="Your name"
						maxLength={20}
						style={{ width: "100%" }}
					/>
				</div>
				<button
					className="btn btn-primary"
					onClick={() => onReady({ name: name.trim() || generatePlayerName() })}
					disabled={!name.trim()}
				>
					Join World
				</button>
			</div>
		</div>
	);
}

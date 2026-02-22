import { useState } from "react";
import type { GameClient } from "../../client.ts";
import { CHUNK_SIZE, WORLD_ID } from "../../../src/actors/open-world/config.ts";

export interface OpenWorldMatchInfo {
	chunkKey: [string, number, number];
	spawnX: number;
	spawnY: number;
	playerName: string;
}

export function OpenWorldMenu({
	client,
	onReady,
	onBack,
}: {
	client: GameClient;
	onReady: (info: OpenWorldMatchInfo) => void;
	onBack: () => void;
}) {
	const [name, setName] = useState(() => `Player#${Math.floor(Math.random() * 10000).toString().padStart(4, "0")}`);
	const [status, setStatus] = useState<"idle" | "loading" | "error">("idle");
	const [error, setError] = useState("");

	const enterWorld = async () => {
		if (!name.trim()) return;
		setStatus("loading");
		setError("");
		const response = resolveChunkForPosition(600, 600);
		onReady({ ...response, playerName: name.trim() });
	};

	return (
		<div className="app">
			<button className="back-link" onClick={onBack}>
				&larr; Back
			</button>
			<div className="menu-container">
				<h2>Open World</h2>
				<p className="menu-description">
					Infinite chunk-based world with cross-chunk movement. Walk beyond
					chunk boundaries to transfer to adjacent chunks. WASD to move.
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
					onClick={() => void enterWorld()}
					disabled={status === "loading" || !name.trim()}
				>
					{status === "loading" ? "Entering..." : "Enter World"}
				</button>
				{status === "error" && (
					<div className="status-box">
						<p className="status-error">{error}</p>
					</div>
				)}
			</div>
		</div>
	);
}

function resolveChunkForPosition(
	x: number,
	y: number,
): { chunkKey: [string, number, number]; spawnX: number; spawnY: number } {
	const chunkX = Math.floor(x / CHUNK_SIZE);
	const chunkY = Math.floor(y / CHUNK_SIZE);
	return {
		chunkKey: [WORLD_ID, chunkX, chunkY],
		spawnX: ((x % CHUNK_SIZE) + CHUNK_SIZE) % CHUNK_SIZE,
		spawnY: ((y % CHUNK_SIZE) + CHUNK_SIZE) % CHUNK_SIZE,
	};
}

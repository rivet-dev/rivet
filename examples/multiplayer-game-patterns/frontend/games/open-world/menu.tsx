import { useState } from "react";
import type { GameClient } from "../../client.ts";

export interface OpenWorldMatchInfo {
	chunkKey: [string, number, number];
	playerId: string;
	playerToken: string;
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
		try {
			const index = client.openWorldIndex.getOrCreate(["main"]).connect();
			const result = await index.send(
				"getChunkForPosition",
				{ x: 600, y: 600, playerName: name.trim() },
				{ wait: true, timeout: 10_000 },
			);
			index.dispose();
			const response = (
				result as { response?: { chunkKey: [string, number, number]; playerId: string; playerToken: string } }
			)?.response;
			if (!response?.chunkKey) throw new Error("Failed to enter world");
			onReady({ ...response, playerName: name.trim() });
		} catch (err) {
			setError(err instanceof Error ? err.message : String(err));
			setStatus("error");
		}
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

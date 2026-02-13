import { useState } from "react";
import type { GameClient } from "../../client.ts";

export interface IoStyleMatchInfo {
	matchId: string;
	playerId: string;
	playerToken: string;
}

export function IoStyleMenu({
	client,
	onReady,
	onBack,
}: {
	client: GameClient;
	onReady: (info: IoStyleMatchInfo) => void;
	onBack: () => void;
}) {
	const [status, setStatus] = useState<"idle" | "matching" | "error">("idle");
	const [error, setError] = useState("");

	const findMatch = async () => {
		setStatus("matching");
		try {
			const mm = client.ioStyleMatchmaker.getOrCreate(["main"]).connect();
			const result = await mm.send("findLobby", {}, { wait: true, timeout: 10_000 });
			mm.dispose();
			const response = (result as { response?: IoStyleMatchInfo })?.response;
			if (!response?.matchId || !response?.playerToken || !response?.playerId) {
				throw new Error("Matchmaker did not return a valid lobby");
			}
			onReady(response);
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
				<h2>IO-Style</h2>
				<p className="menu-description">
					Open lobby matchmaking with server-authoritative movement. Use WASD or arrow
					keys to move your player around the world.
				</p>
				<button
					className="btn btn-primary"
					onClick={() => void findMatch()}
					disabled={status === "matching"}
				>
					{status === "matching" ? "Finding match..." : "Find Match"}
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

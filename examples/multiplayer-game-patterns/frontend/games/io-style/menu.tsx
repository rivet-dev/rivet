import { useState } from "react";
import type { GameClient } from "../../client.ts";

export interface IoStyleMatchInfo {
	matchId: string;
	playerId: string;
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

	// docs:start find-lobby
	const findMatch = async () => {
		setStatus("matching");
		try {
			// A plain action call needs no connection, because the player only
			// talks to the match from here on.
			const lobby = await client.ioStyleMatchmaker
				.getOrCreate(["main"])
				.findLobby();
			onReady(lobby);
		} catch (err) {
			setError(err instanceof Error ? err.message : String(err));
			setStatus("error");
		}
	};
	// docs:end find-lobby

	return (
		<div className="app">
			<button className="back-link" onClick={onBack}>
				&larr; Back
			</button>
			<div className="menu-container">
				<h2>IO-Style</h2>
				<p className="menu-description">
					Open lobby matchmaking with server-authoritative movement.
					Use WASD or arrow keys to move your player around the world.
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

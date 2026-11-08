import { useState } from "react";
import type { GameClient } from "../../client.ts";

export interface BattleRoyaleMatchInfo {
	matchId: string;
	playerId: string;
}

export function BattleRoyaleMenu({
	client,
	onReady,
	onBack,
}: {
	client: GameClient;
	onReady: (info: BattleRoyaleMatchInfo) => void;
	onBack: () => void;
}) {
	const [status, setStatus] = useState<"idle" | "matching" | "error">("idle");
	const [error, setError] = useState("");

	// docs:start find-match
	const findMatch = async () => {
		setStatus("matching");
		try {
			// A plain action call needs no connection, because the player only
			// talks to the match from here on.
			const match = await client.battleRoyaleMatchmaker
				.getOrCreate(["main"])
				.findMatch();
			onReady(match);
		} catch (err) {
			setError(err instanceof Error ? err.message : String(err));
			setStatus("error");
		}
	};
	// docs:end find-match

	return (
		<div className="app">
			<button className="back-link" onClick={onBack}>
				&larr; Back
			</button>
			<div className="menu-container">
				<h2>Battle Royale</h2>
				<p className="menu-description">
					Join a lobby, wait for players, then fight in a shrinking
					zone. Last one standing wins. WASD to move, click to shoot.
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

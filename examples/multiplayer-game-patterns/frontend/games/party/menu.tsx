import { useState } from "react";
import type { GameClient } from "../../client.ts";

export interface PartyMatchInfo {
	matchId: string;
	playerId: string;
	playerToken: string;
	partyCode: string;
}

export function PartyMenu({
	client,
	onReady,
	onBack,
}: {
	client: GameClient;
	onReady: (info: PartyMatchInfo) => void;
	onBack: () => void;
}) {
	const [joinCode, setJoinCode] = useState("");
	const [status, setStatus] = useState<"idle" | "loading" | "error">("idle");
	const [error, setError] = useState("");

	const createParty = async () => {
		setStatus("loading");
		setError("");
		try {
			const mm = client.partyMatchmaker.getOrCreate(["main"]).connect();
			const result = await mm.send(
				"createParty",
				{},
				{ wait: true, timeout: 10_000 },
			);
			mm.dispose();
			const response = (result as { response?: PartyMatchInfo })?.response;
			if (!response?.matchId) throw new Error("Failed to create party");
			onReady(response);
		} catch (err) {
			setError(err instanceof Error ? err.message : String(err));
			setStatus("error");
		}
	};

	const joinParty = async () => {
		if (!joinCode.trim()) return;
		setStatus("loading");
		setError("");
		try {
			const mm = client.partyMatchmaker.getOrCreate(["main"]).connect();
			const result = await mm.send(
				"joinParty",
				{ partyCode: joinCode.trim() },
				{ wait: true, timeout: 10_000 },
			);
			mm.dispose();
			const response = (
				result as { response?: { matchId: string; playerId: string; playerToken: string } }
			)?.response;
			if (!response?.matchId) throw new Error("Failed to join party");
			onReady({ ...response, partyCode: joinCode.trim().toUpperCase() });
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
				<h2>Party</h2>
				<p className="menu-description">
					Create or join a party lobby with invite codes. Host controls the game
					flow.
				</p>

				<button
					className="btn btn-primary"
					onClick={() => void createParty()}
					disabled={status === "loading"}
					style={{ width: "100%", marginBottom: 20 }}
				>
					{status === "loading" ? "Loading..." : "Create Party"}
				</button>

				<div style={{ display: "flex", alignItems: "center", gap: 12, marginBottom: 20 }}>
					<div style={{ flex: 1, height: 1, background: "#2c2c2e" }} />
					<span style={{ color: "#8e8e93", fontSize: 12 }}>OR JOIN WITH CODE</span>
					<div style={{ flex: 1, height: 1, background: "#2c2c2e" }} />
				</div>

				<input
					type="text"
					placeholder="Party code"
					value={joinCode}
					onChange={(e) => setJoinCode(e.target.value.toUpperCase())}
					className="text-input"
					maxLength={6}
					style={{
						width: "100%",
						marginBottom: 12,
						fontFamily: "ui-monospace, SFMono-Regular, 'SF Mono', Consolas, monospace",
						fontSize: 18,
						letterSpacing: 4,
						textAlign: "center",
					}}
				/>
				<button
					className="btn btn-primary"
					onClick={() => void joinParty()}
					disabled={status === "loading" || !joinCode.trim()}
					style={{ width: "100%" }}
				>
					{status === "loading" ? "Loading..." : "Join Party"}
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

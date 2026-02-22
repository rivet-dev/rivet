import { useState } from "react";
import type { GameClient } from "../../client.ts";

export interface TurnBasedMatchInfo {
	matchId: string;
	playerId: string;
	playerToken: string;
	inviteCode?: string;
}

export function TurnBasedMenu({
	client,
	onReady,
	onBack,
}: {
	client: GameClient;
	onReady: (info: TurnBasedMatchInfo) => void;
	onBack: () => void;
}) {
	const [name, setName] = useState(() => `Player#${Math.floor(Math.random() * 10000).toString().padStart(4, "0")}`);
	const [joinCode, setJoinCode] = useState("");
	const [status, setStatus] = useState<"idle" | "loading" | "error">("idle");
	const [error, setError] = useState("");

	const findMatch = async () => {
		if (!name.trim()) return;
		setStatus("loading");
		setError("");
		try {
			const mm = client.turnBasedMatchmaker.getOrCreate(["main"]).connect();
			const result = await mm.send(
				"findMatch",
				{ playerName: name.trim() },
				{ wait: true, timeout: 10_000 },
			);
			mm.dispose();
			const response = (
				result as { response?: TurnBasedMatchInfo }
			)?.response;
			if (!response?.matchId) throw new Error("Failed to find match");
			onReady(response);
		} catch (err) {
			setError(err instanceof Error ? err.message : String(err));
			setStatus("error");
		}
	};

	const createGame = async () => {
		if (!name.trim()) return;
		setStatus("loading");
		setError("");
		try {
			const mm = client.turnBasedMatchmaker.getOrCreate(["main"]).connect();
			const result = await mm.send(
				"createGame",
				{ playerName: name.trim() },
				{ wait: true, timeout: 10_000 },
			);
			mm.dispose();
			const response = (result as { response?: TurnBasedMatchInfo })?.response;
			if (!response?.matchId) throw new Error("Failed to create game");
			onReady(response);
		} catch (err) {
			setError(err instanceof Error ? err.message : String(err));
			setStatus("error");
		}
	};

	const joinByCode = async () => {
		if (!name.trim() || !joinCode.trim()) return;
		setStatus("loading");
		setError("");
		try {
			const mm = client.turnBasedMatchmaker.getOrCreate(["main"]).connect();
			const result = await mm.send(
				"joinByCode",
				{ inviteCode: joinCode.trim(), playerName: name.trim() },
				{ wait: true, timeout: 10_000 },
			);
			mm.dispose();
			const response = (
				result as { response?: { matchId: string; playerId: string; playerToken: string } }
			)?.response;
			if (!response?.matchId) throw new Error("Failed to join game");
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
				<h2>Turn-Based</h2>
				<p className="menu-description">
					Tic-tac-toe with invite codes and open matchmaking. Find an open match,
					create a private game, or join by code.
				</p>

				<div style={{ marginBottom: 16 }}>
					<label style={{ display: "block", color: "#8e8e93", fontSize: 12, marginBottom: 4 }}>
						Your Name
					</label>
					<input
						type="text"
						placeholder="Your name"
						value={name}
						onChange={(e) => setName(e.target.value)}
						className="text-input"
						style={{ width: "100%" }}
					/>
				</div>

				<div style={{ height: 1, background: "#2c2c2e", marginBottom: 16 }} />

				<div style={{ display: "flex", gap: 16 }}>
					<div style={{ flex: 1, display: "flex", flexDirection: "column", gap: 8 }}>
						<div style={{ color: "#8e8e93", fontSize: 12, fontWeight: 600, textTransform: "uppercase", letterSpacing: 1 }}>
							Quick Play
						</div>
						<button
							className="btn btn-primary"
							onClick={() => void findMatch()}
							disabled={status === "loading" || !name.trim()}
							style={{ width: "100%" }}
						>
							{status === "loading" ? "Loading..." : "Find Match"}
						</button>
						<button
							className="btn btn-secondary"
							onClick={() => void createGame()}
							disabled={status === "loading" || !name.trim()}
							style={{ width: "100%" }}
						>
							{status === "loading" ? "Loading..." : "Create Private"}
						</button>
					</div>

					<div style={{ width: 1, background: "#2c2c2e" }} />

					<div style={{ flex: 1, display: "flex", flexDirection: "column", gap: 8 }}>
						<div style={{ color: "#8e8e93", fontSize: 12, fontWeight: 600, textTransform: "uppercase", letterSpacing: 1 }}>
							Join by Code
						</div>
						<input
							type="text"
							placeholder="Invite code"
							value={joinCode}
							onChange={(e) => setJoinCode(e.target.value.toUpperCase())}
							className="text-input"
							maxLength={6}
							style={{
								width: "100%",
								fontFamily: "ui-monospace, SFMono-Regular, 'SF Mono', Consolas, monospace",
								fontSize: 16,
								letterSpacing: 3,
								textAlign: "center",
							}}
						/>
						<button
							className="btn btn-primary"
							onClick={() => void joinByCode()}
							disabled={status === "loading" || !name.trim() || !joinCode.trim()}
							style={{ width: "100%" }}
						>
							{status === "loading" ? "Loading..." : "Join Game"}
						</button>
					</div>
				</div>

				{status === "error" && (
					<div className="status-box">
						<p className="status-error">{error}</p>
					</div>
				)}
			</div>
		</div>
	);
}

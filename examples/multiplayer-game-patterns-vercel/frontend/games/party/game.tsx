import { useCallback, useEffect, useRef, useState } from "react";
import type { GameClient } from "../../client.ts";
import type { PartyMatchInfo } from "./menu.tsx";
import { PartyBot } from "./bot.ts";

interface PartySnapshot {
	matchId: string;
	partyCode: string;
	phase: "waiting" | "playing" | "finished";
	members: Record<string, { name: string; color: string; isHost: boolean; isReady: boolean; connected: boolean }>;
}

export function PartyGame({
	client,
	matchInfo,
	onLeave,
}: {
	client: GameClient;
	matchInfo: PartyMatchInfo;
	onLeave: () => void;
}) {
	const [snapshot, setSnapshot] = useState<PartySnapshot | null>(null);
	const [nameInput, setNameInput] = useState(matchInfo.playerName || "Player");
	// biome-ignore lint/suspicious/noExplicitAny: connection handle
	const connRef = useRef<any>(null);
	const botsRef = useRef<PartyBot[]>([]);
	const nameTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

	const cleanup = useCallback(() => {
		const conn = connRef.current;
		connRef.current = null;
		conn?.dispose();
	}, []);

	useEffect(() => {
		const conn = client.partyMatch
			.get([matchInfo.matchId], {
				params: {
					playerId: matchInfo.playerId,
					joinToken: matchInfo.joinToken,
				},
			})
			.connect();
		connRef.current = conn;

		conn.on("partyUpdate", (raw: unknown) => {
			setSnapshot(raw as PartySnapshot);
		});

		conn.getSnapshot().then((snap: unknown) => {
			const s = snap as PartySnapshot;
			setSnapshot(s);
			const myName = s.members[matchInfo.playerId]?.name;
			if (myName) setNameInput(myName);
		});

		return () => {
			cleanup();
			for (const bot of botsRef.current) bot.destroy();
			botsRef.current = [];
		};
	}, [client, matchInfo, cleanup]);

	const onNameChange = (value: string) => {
		setNameInput(value);
		if (nameTimeoutRef.current) clearTimeout(nameTimeoutRef.current);
		nameTimeoutRef.current = setTimeout(() => {
			connRef.current?.setName({ name: value }).catch(() => {});
		}, 300);
	};

	const addBot = () => {
		botsRef.current.push(new PartyBot(client, matchInfo.partyCode));
	};

	const toggleReady = () => {
		connRef.current?.toggleReady().catch(() => {});
	};

	const startGame = () => {
		connRef.current?.startGame().catch(() => {});
	};

	const finishGame = () => {
		connRef.current?.finishGame().catch(() => {});
	};

	const myMember = snapshot?.members[matchInfo.playerId];
	const isHost = myMember?.isHost ?? false;
	const memberList = snapshot ? Object.entries(snapshot.members) : [];

	return (
		<div className="app">
			<div className="game-header">
				<h2>Party</h2>
				<div className="btn-row">
					<button className="btn btn-secondary" onClick={addBot}>Add Bot</button>
					<button className="btn btn-secondary" onClick={() => { cleanup(); onLeave(); }}>Leave</button>
				</div>
			</div>

			<div className="menu-container">
				<div className="party-code-display">
					<div className="party-code-label">Party Code</div>
					<div className="party-code">{matchInfo.partyCode}</div>
				</div>

				{snapshot && (
					<div className="party-phase-badge">
						<span className={`badge badge-${snapshot.phase}`}>
							{snapshot.phase.toUpperCase()}
						</span>
					</div>
				)}

				<div style={{ marginBottom: 16 }}>
					<label style={{ display: "block", color: "#8e8e93", fontSize: 12, marginBottom: 4 }}>
						Your Name
					</label>
					<input
						type="text"
						placeholder="Your name"
						value={nameInput}
						onChange={(e) => onNameChange(e.target.value)}
						className="text-input"
						style={{ width: "100%" }}
					/>
				</div>

				<div className="party-members">
					<div className="party-members-header">
						Members ({memberList.length})
					</div>
					{memberList.map(([id, member]) => (
						<div key={id} className="party-member-row">
							<span className="party-member-name" style={{ color: member.color }}>
								{member.name}
								{id === matchInfo.playerId ? " (You)" : ""}
							</span>
							<span className="party-member-badges">
								{member.isHost && <span className="badge badge-live">Host</span>}
								{member.isReady && <span className="badge badge-live">Ready</span>}
								{!member.connected && <span className="badge badge-finished">Offline</span>}
							</span>
						</div>
					))}
				</div>

				{snapshot?.phase === "waiting" && (
					<div className="btn-row" style={{ marginTop: 16 }}>
						<button
							className={`btn ${myMember?.isReady ? "btn-secondary" : "btn-success"}`}
							onClick={toggleReady}
						>
							{myMember?.isReady ? "Unready" : "Ready"}
						</button>
						{isHost && (
							<button className="btn btn-primary" onClick={startGame}>
								Start Game
							</button>
						)}
					</div>
				)}

				{snapshot?.phase === "playing" && (
					<div style={{ marginTop: 16, textAlign: "center" }}>
						<p style={{ color: "#8e8e93", fontSize: 14, marginBottom: 12 }}>
							Game is in progress
						</p>
						{isHost ? (
							<button className="btn btn-primary" onClick={finishGame}>
								Finish Game
							</button>
						) : (
							<p style={{ color: "#6e6e73", fontSize: 12 }}>
								The host can finish the game when ready.
							</p>
						)}
					</div>
				)}

				{snapshot?.phase === "finished" && (
					<div className="match-found-text" style={{ textAlign: "center", marginTop: 16 }}>
						Game Complete!
					</div>
				)}
			</div>
		</div>
	);
}

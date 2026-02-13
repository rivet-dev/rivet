import { useCallback, useEffect, useRef, useState } from "react";
import type { GameClient } from "../../client.ts";
import type { TurnBasedMatchInfo } from "./menu.tsx";
import type { CellValue, GameResult } from "../../../src/actors/turn-based/config.ts";
import { TurnBasedBot } from "./bot.ts";

interface GameSnapshot {
	matchId: string;
	board: CellValue[][];
	currentTurn: "X" | "O";
	result: GameResult;
	moveCount: number;
	players: Record<string, { name: string; symbol: "X" | "O"; connected: boolean }>;
}

export function TurnBasedGame({
	client,
	matchInfo,
	onLeave,
}: {
	client: GameClient;
	matchInfo: TurnBasedMatchInfo;
	onLeave: () => void;
}) {
	const [snapshot, setSnapshot] = useState<GameSnapshot | null>(null);
	// biome-ignore lint/suspicious/noExplicitAny: connection handle
	const connRef = useRef<any>(null);
	const botsRef = useRef<TurnBasedBot[]>([]);

	const cleanup = useCallback(() => {
		const conn = connRef.current;
		connRef.current = null;
		conn?.dispose();
	}, []);

	useEffect(() => {
		const conn = client.turnBasedMatch
			.get([matchInfo.matchId], { params: { playerToken: matchInfo.playerToken } })
			.connect();
		connRef.current = conn;

		conn.on("gameUpdate", (raw: unknown) => {
			setSnapshot(raw as GameSnapshot);
		});

		conn.getSnapshot().then((snap: unknown) => {
			setSnapshot(snap as GameSnapshot);
		});

		return () => {
			cleanup();
			for (const bot of botsRef.current) bot.destroy();
			botsRef.current = [];
		};
	}, [client, matchInfo, cleanup]);

	const addBot = () => {
		if (matchInfo.inviteCode) {
			botsRef.current.push(new TurnBasedBot(client, matchInfo.inviteCode));
		}
	};

	const makeMove = (row: number, col: number) => {
		connRef.current?.makeMove({ row, col }).catch(() => {});
	};

	const myPlayer = snapshot?.players[matchInfo.playerId];
	const mySymbol = myPlayer?.symbol;
	const isMyTurn = mySymbol === snapshot?.currentTurn;
	const playerEntries = snapshot ? Object.entries(snapshot.players) : [];
	const waitingForOpponent = playerEntries.length < 2;

	const resultText = snapshot?.result === "draw"
		? "Draw!"
		: snapshot?.result === "x_wins"
			? (mySymbol === "X" ? "You Win!" : "You Lose!")
			: snapshot?.result === "o_wins"
				? (mySymbol === "O" ? "You Win!" : "You Lose!")
				: null;

	return (
		<div className="app">
			<div className="game-header">
				<h2>Turn-Based</h2>
				<div className="btn-row">
					{waitingForOpponent && matchInfo.inviteCode && (
						<button className="btn btn-secondary" onClick={addBot}>Add Bot</button>
					)}
					<button className="btn btn-secondary" onClick={() => { cleanup(); onLeave(); }}>Leave</button>
				</div>
			</div>

			<div className="menu-container">
				<div style={{ textAlign: "center", marginBottom: 16 }}>
					{playerEntries.map(([id, p]) => (
						<span key={id} style={{ margin: "0 12px", fontSize: 14 }}>
							<strong>{p.symbol}</strong>: {p.name}
							{id === matchInfo.playerId ? " (You)" : ""}
						</span>
					))}
				</div>

				{waitingForOpponent ? (
					<div className="queue-status">
						{matchInfo.inviteCode && (
							<div className="party-code-display" style={{ marginBottom: 16 }}>
								<div className="party-code-label">Invite Code</div>
								<div className="party-code">{matchInfo.inviteCode}</div>
							</div>
						)}
						<div className="queue-label">Waiting for opponent...</div>
					</div>
				) : (
					<>
						<div style={{ textAlign: "center", marginBottom: 12 }}>
							{snapshot?.result === null ? (
								<span style={{ color: isMyTurn ? "#30d158" : "#8e8e93" }}>
									{isMyTurn ? "Your turn!" : "Opponent's turn..."}
								</span>
							) : (
								<span style={{
									color: resultText === "You Win!" ? "#30d158" : resultText === "Draw!" ? "#ff4f00" : "#ff3b30",
									fontSize: 20,
									fontWeight: 700,
								}}>
									{resultText}
								</span>
							)}
						</div>

						{snapshot && (
							<div className="ttt-board">
								{snapshot.board.map((row, r) =>
									row.map((cell, c) => (
										<button
											key={`${r}-${c}`}
											className={`ttt-cell ${cell ? "ttt-cell-filled" : ""} ${cell === "X" ? "ttt-cell-x" : cell === "O" ? "ttt-cell-o" : ""}`}
											onClick={() => makeMove(r, c)}
											disabled={!!cell || !isMyTurn || snapshot.result !== null}
										>
											{cell}
										</button>
									)),
								)}
							</div>
						)}
					</>
				)}
			</div>
		</div>
	);
}

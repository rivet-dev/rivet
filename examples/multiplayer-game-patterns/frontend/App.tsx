import { useEffect, useMemo, useState } from "react";
import { createClient } from "rivetkit/client";
import type { registry } from "../src/actors/index.ts";

type DemoType =
	| "io-style"
	| "competitive"
	| "party"
	| "async-turn-based"
	| "ranked"
	| "battle-royale";

type DemoState = {
	running: boolean;
	logs: string[];
};

const DEMOS: Array<{ type: DemoType; title: string; description: string }> = [
	{
		type: "io-style",
		title: "io-style",
		description: "Open lobby matchmaking and a 10 tps room scaffold.",
	},
	{
		type: "competitive",
		title: "competitive",
		description: "Filled-room queue with mode selection and team assignment at 20 tps.",
	},
	{
		type: "party",
		title: "party",
		description: "Host-created party code flow with no tick loop.",
	},
	{
		type: "async-turn-based",
		title: "async turn-based",
		description: "Invite and open pool pairing with turn-based state transitions.",
	},
	{
		type: "ranked",
		title: "ranked",
		description: "ELO-based matchmaking at 20 tps with post-match rating updates.",
	},
	{
		type: "battle-royale",
		title: "battle royale",
		description: "Queue-threshold start with a 10 tps battle royale match scaffold.",
	},
];

const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

async function waitFor(
	fn: () => Promise<unknown>,
	timeoutMs = 3000,
): Promise<any> {
	const start = Date.now();
	while (Date.now() - start < timeoutMs) {
		const value = await fn();
		if (value != null) return value;
		await sleep(25);
	}
	throw new Error("timed out waiting for value");
}

async function disposeAll(items: Array<{ dispose: () => Promise<void> } | undefined>) {
	await Promise.all(
		items
			.filter((item): item is { dispose: () => Promise<void> } => Boolean(item))
			.map((item) => item.dispose().catch(() => undefined)),
	);
}

function nowTime() {
	return new Date().toLocaleTimeString();
}

export function App() {
	const client = useMemo(
		() =>
			createClient<typeof registry>({
				endpoint: `${window.location.origin}/api/rivet`,
				namespace: "default",
				runnerName: "default",
			}),
		[],
	);

	const [states, setStates] = useState<Record<DemoType, DemoState>>({
		"io-style": { running: false, logs: [] },
		competitive: { running: false, logs: [] },
		party: { running: false, logs: [] },
		"async-turn-based": { running: false, logs: [] },
		ranked: { running: false, logs: [] },
		"battle-royale": { running: false, logs: [] },
	});

	useEffect(() => {
		return () => {
			void client.dispose();
		};
	}, [client]);

	const appendLog = (type: DemoType, message: string) => {
		setStates((prev) => {
			const existing = prev[type];
			const nextLogs = [`${nowTime()} - ${message}`, ...existing.logs].slice(0, 8);
			return { ...prev, [type]: { ...existing, logs: nextLogs } };
		});
	};

	const setRunning = (type: DemoType, running: boolean) => {
		setStates((prev) => ({ ...prev, [type]: { ...prev[type], running } }));
	};

	const runDemo = async (type: DemoType) => {
		setRunning(type, true);
		setStates((prev) => ({ ...prev, [type]: { ...prev[type], logs: [] } }));

			try {
				if (type === "io-style") {
					const mm = client.ioStyleMatchmaker.getOrCreate(["main"]);
					const firstPlayerId = `io-a-${Date.now()}`;
					const secondPlayerId = `io-b-${Date.now()}`;
					await mm.queue.findOpenLobby.send({ playerId: firstPlayerId });
					await mm.queue.findOpenLobby.send({ playerId: secondPlayerId });
					const first = await waitFor(() => mm.getLobbyForPlayer({ playerId: firstPlayerId }));
					const second = await waitFor(() =>
						mm.getLobbyForPlayer({ playerId: secondPlayerId }),
					);
					appendLog(type, `matchmaker resolved room ${first.matchId}`);

					const a = client.ioStyleMatch
						.getOrCreate([first.matchId], { params: { playerToken: first.playerToken } })
						.connect();
					const b = client.ioStyleMatch
						.getOrCreate([first.matchId], { params: { playerToken: second.playerToken } })
						.connect();
					await sleep(250);

				const snapshot = await a.getSnapshot();
				appendLog(type, `room is ${snapshot.phase} at tick ${snapshot.tick} with ${snapshot.playerCount} players`);
				await disposeAll([a, b]);
			}

			if (type === "competitive") {
				const mm = client.competitiveMatchmaker.getOrCreate(["main"]);
				const players = ["comp-a", "comp-b", "comp-c", "comp-d"];
				for (const playerId of players) {
					await mm.queue.queueForMatch.send({ playerId, mode: "duo" });
					appendLog(type, `${playerId} queued`);
				}

					const assigned = await waitFor(() => mm.getAssignment({ playerId: players[0] }));
					appendLog(type, `filled match ${assigned.matchId} formed for mode ${assigned.mode}`);
					const assignments = await Promise.all(
						players.map((playerId) => waitFor(() => mm.getAssignment({ playerId }))),
					);
					const playerTokenByPlayerId = new Map(
						assignments
							.filter(
								(entry): entry is NonNullable<(typeof assignments)[number]> =>
									entry != null,
							)
							.map((entry) => [entry.playerId, entry.playerToken]),
					);

					const conns = players.map((playerId) =>
						client.competitiveMatch
							.getOrCreate([assigned.matchId], {
								params: { playerToken: playerTokenByPlayerId.get(playerId)! },
							})
							.connect(),
					);

				await sleep(120);
				const live = await conns[0]!.getSnapshot();
				appendLog(type, `match is ${live.phase} at tick ${live.tick}`);
				await conns[0]!.finish({ winnerTeam: 0 });
				appendLog(type, "match finished and assignment cleaned up");
				await disposeAll([...conns]);
			}

				if (type === "party") {
					const mm = client.partyMatchmaker.getOrCreate(["main"]);
					const hostPlayerId = "party-host";
					await mm.queue.createParty.send({ hostPlayerId });
					const created = await waitFor(() => mm.getPartyForHost({ hostPlayerId }));
					appendLog(type, `created party code ${created.partyCode}`);

					await mm.queue.joinParty.send({ partyCode: created.partyCode, playerId: "party-guest" });
					const joined = await waitFor(() =>
						mm.getJoinByPlayer({ partyCode: created.partyCode, playerId: "party-guest" }),
					);
					const host = client.partyMatch
						.getOrCreate([created.matchId], {
							params: { playerToken: created.hostPlayerToken },
						})
						.connect();
					const guest = client.partyMatch
						.getOrCreate([created.matchId], { params: { playerToken: joined.playerToken } })
						.connect();
					const match = host;
					await match.start();
					const snapshot = await match.getSnapshot();
					appendLog(type, `party phase is ${snapshot.phase}`);
					await disposeAll([match, guest]);
				}

					if (type === "async-turn-based") {
						const mm = client.asyncTurnBasedMatchmaker.getOrCreate(["main"]);
						const inviteCode = `invite-${Date.now()}`;
						await mm.queue.createInvite.send({
							inviteCode,
							fromPlayerId: "turn-a",
							toPlayerId: "turn-b",
						});
						await mm.queue.acceptInvite.send({ inviteCode, playerId: "turn-b" });
						const accepted = await waitFor(() => mm.getAssignment({ playerId: "turn-b" }));
						const turnA = await waitFor(() => mm.getAssignment({ playerId: "turn-a" }));

						appendLog(type, `invite accepted into ${accepted.matchId}`);
						const a = client.asyncTurnBasedMatch
							.getOrCreate([accepted.matchId], { params: { playerToken: turnA.playerToken } })
							.connect();
						const b = client.asyncTurnBasedMatch
							.getOrCreate([accepted.matchId], { params: { playerToken: accepted.playerToken } })
							.connect();
					await a.submitTurn({ move: "open" });
					await b.submitTurn({ move: "reply" });
					await a.finish({ winnerPlayerId: "turn-a" });
					appendLog(type, "turn sequence completed and match closed");
					await disposeAll([a, b]);
				}

			if (type === "ranked") {
				const mm = client.rankedMatchmaker.getOrCreate(["main"]);
				await mm.queue.debugSetRating.send({ playerId: "rank-a", elo: 1200 });
				await mm.queue.debugSetRating.send({ playerId: "rank-b", elo: 1210 });
					await mm.queue.queueForMatch.send({ playerId: "rank-a" });
					await mm.queue.queueForMatch.send({ playerId: "rank-b" });
					const assigned = await waitFor(() => mm.getAssignment({ playerId: "rank-a" }));
					const assignedB = await waitFor(() => mm.getAssignment({ playerId: "rank-b" }));

					const a = client.rankedMatch
						.getOrCreate([assigned.matchId], { params: { playerToken: assigned.playerToken } })
						.connect();
					const b = client.rankedMatch
						.getOrCreate([assigned.matchId], { params: { playerToken: assignedB.playerToken } })
						.connect();
					await sleep(120);
					await a.finish({ winnerPlayerId: "rank-a" });
				const aRating = await mm.getRating({ playerId: "rank-a" });
				const bRating = await mm.getRating({ playerId: "rank-b" });
				appendLog(type, `updated ratings -> rank-a: ${aRating.elo}, rank-b: ${bRating.elo}`);
				await disposeAll([a, b]);
			}

			if (type === "battle-royale") {
				const mm = client.battleRoyaleMatchmaker.getOrCreate(["main"]);
					await mm.queue.joinQueue.send({ playerId: "br-a" });
					await mm.queue.joinQueue.send({ playerId: "br-b" });
					await mm.queue.joinQueue.send({ playerId: "br-c" });
					const assigned = await waitFor(() => mm.getAssignment({ playerId: "br-a" }));
					const assignedB = await waitFor(() => mm.getAssignment({ playerId: "br-b" }));
					const assignedC = await waitFor(() => mm.getAssignment({ playerId: "br-c" }));

					appendLog(type, `battle royale match ${assigned.matchId} created`);
					const a = client.battleRoyaleMatch
						.getOrCreate([assigned.matchId], { params: { playerToken: assigned.playerToken } })
						.connect();
					const b = client.battleRoyaleMatch
						.getOrCreate([assigned.matchId], { params: { playerToken: assignedB.playerToken } })
						.connect();
					const cConn = client.battleRoyaleMatch
						.getOrCreate([assigned.matchId], { params: { playerToken: assignedC.playerToken } })
						.connect();
				await a.startNow();
				await sleep(220);
				const live = await a.getSnapshot();
				appendLog(type, `active tick ${live.tick}, zone radius ${live.zoneRadius.toFixed(2)}`);
				await a.eliminate({ victimPlayerId: "br-b" });
				await a.eliminate({ victimPlayerId: "br-c" });
				const final = await a.getSnapshot();
				appendLog(type, `winner: ${final.winnerPlayerId}`);
				await disposeAll([a, b, cConn]);
			}
		} catch (err) {
			appendLog(type, `error: ${err instanceof Error ? err.message : String(err)}`);
		} finally {
			setRunning(type, false);
		}
	};

	return (
		<div style={{ maxWidth: 1000, margin: "0 auto", padding: 24, fontFamily: "sans-serif" }}>
			<h1 style={{ marginTop: 0 }}>Matchmaking and Session Patterns</h1>
			<p style={{ color: "#555", marginTop: 0 }}>
				This UI runs scripted golden-path demos for each matchmaking pattern. It does not render gameplay.
			</p>

			<div style={{ display: "grid", gap: 16 }}>
				{DEMOS.map((demo) => {
					const state = states[demo.type];
					return (
						<section key={demo.type} style={{ border: "1px solid #ddd", borderRadius: 8, padding: 16 }}>
							<div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
								<div>
									<h2 style={{ margin: 0, fontSize: 20 }}>{demo.title}</h2>
									<p style={{ margin: "6px 0 0", color: "#666" }}>{demo.description}</p>
								</div>
								<button
									onClick={() => void runDemo(demo.type)}
									disabled={state.running}
									style={{
										padding: "8px 12px",
										borderRadius: 6,
										border: "1px solid #aaa",
										background: state.running ? "#f2f2f2" : "#fff",
										cursor: state.running ? "default" : "pointer",
									}}
								>
									{state.running ? "Running..." : "Run demo"}
								</button>
							</div>

							<div style={{ marginTop: 12, fontSize: 13, color: "#333" }}>
								{state.logs.length === 0 ? (
									<p style={{ margin: 0, color: "#777" }}>No run yet.</p>
								) : (
									<ul style={{ margin: 0, paddingLeft: 18 }}>
										{state.logs.map((line) => (
											<li key={line} style={{ marginBottom: 4 }}>
												{line}
											</li>
										))}
									</ul>
								)}
							</div>
						</section>
					);
				})}
			</div>
		</div>
	);
}

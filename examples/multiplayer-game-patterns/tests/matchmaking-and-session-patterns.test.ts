import { setupTest } from "rivetkit/test";
import { describe, expect, test } from "vitest";
import { registry } from "../src/actors/index.ts";

const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

async function waitFor(fn: () => Promise<unknown>, timeoutMs = 3000): Promise<any> {
	const start = Date.now();
	while (Date.now() - start < timeoutMs) {
		const value = await fn();
		if (value != null) return value;
		await sleep(25);
	}
	throw new Error("timed out waiting for value");
}

async function waitUntil(fn: () => Promise<boolean>, timeoutMs = 3000): Promise<void> {
	const start = Date.now();
	while (Date.now() - start < timeoutMs) {
		if (await fn()) return;
		await sleep(25);
	}
	throw new Error("timed out waiting for condition");
}

describe("matchmaking and session patterns", () => {
	test("io-style open lobby + 10 tps match", async (ctx) => {
		const { client } = await setupTest(ctx, registry);

		const mm = client.ioStyleMatchmaker.getOrCreate(["main"]).connect();
		const firstPlayerId = "io-a";
		const secondPlayerId = "io-b";
		await mm.queue.findOpenLobby.send({ playerId: firstPlayerId });
		await mm.queue.findOpenLobby.send({ playerId: secondPlayerId });

		const first = await waitFor(() => mm.getLobbyForPlayer({ playerId: firstPlayerId }));
		const second = await waitFor(() => mm.getLobbyForPlayer({ playerId: secondPlayerId }));
		expect(second.matchId).toBe(first.matchId);

		const a = client.ioStyleMatch
			.getOrCreate([first.matchId], { params: { playerToken: first.playerToken } })
			.connect();
		const b = client.ioStyleMatch
			.getOrCreate([first.matchId], { params: { playerToken: second.playerToken } })
			.connect();
		await sleep(260);

		const snapshot = await a.getSnapshot();
		expect(snapshot.playerCount).toBe(2);
		expect(snapshot.tick).toBeGreaterThanOrEqual(2);
		expect(snapshot.phase).toBe("live");

		await Promise.all([a.dispose(), b.dispose(), mm.dispose()]);
	}, 15_000);

	test("competitive filled-room + team assignment + 20 tps", async (ctx) => {
		const { client } = await setupTest(ctx, registry);

		const mm = client.competitiveMatchmaker.getOrCreate(["main"]).connect();
		const players = ["comp-a", "comp-b", "comp-c", "comp-d"];

		for (const playerId of players) {
			await mm.queue.queueForMatch.send({ playerId, mode: "duo" });
		}

		const assignment = await waitFor(() => mm.getAssignment({ playerId: players[0]! }));
		if (!assignment) throw new Error("missing competitive assignment");
		const matchId = assignment.matchId;
		expect(matchId).toMatch(/^competitive-duo-/);

		const assignments = await Promise.all(
			players.map(async (playerId) => {
				const assignment = await waitFor(() => mm.getAssignment({ playerId }));
				if (!assignment) {
					throw new Error(`missing competitive assignment for ${playerId}`);
				}
				return assignment;
			}),
		);
		const playerTokenByPlayerId = new Map(
			assignments.map((entry) => [entry.playerId, entry.playerToken]),
		);

		const conns = players.map((playerId) =>
			client.competitiveMatch
				.getOrCreate([matchId], { params: { playerToken: playerTokenByPlayerId.get(playerId)! } })
				.connect(),
		);

		const joined = await waitFor(async () => {
			const snapshot = await conns[0]!.getSnapshot();
			return snapshot.phase === "live" ? snapshot : null;
		});
		expect(joined.phase).toBe("live");

		const teamCounts: Record<string, number> = {};
		for (const player of Object.values(joined.players as Record<string, { teamId: number }>)) {
			const key = String(player.teamId);
			teamCounts[key] = (teamCounts[key] ?? 0) + 1;
		}
		expect(teamCounts["0"]).toBe(2);
		expect(teamCounts["1"]).toBe(2);

		await sleep(120);
		const liveTick = await conns[0]!.getSnapshot();
		expect(liveTick.tick).toBeGreaterThanOrEqual(2);

		await conns[0]!.finish({ winnerTeam: 0 });
		const finished = await conns[1]!.getSnapshot();
		expect(finished.phase).toBe("finished");

		await waitUntil(async () => (await mm.getAssignment({ playerId: players[0]! })) == null);

		await Promise.all([...conns.map((conn) => conn.dispose()), mm.dispose()]);
	}, 15_000);

	test("party host start + party code with no tick loop", async (ctx) => {
		const { client } = await setupTest(ctx, registry);

		const mm = client.partyMatchmaker.getOrCreate(["main"]).connect();
		const hostPlayerId = "party-host";
		await mm.queue.createParty.send({ hostPlayerId });
		const created = await waitFor(() => mm.getPartyForHost({ hostPlayerId }));
		expect(created.partyCode.length).toBe(6);

		const guestPlayerId = "party-guest";
		await mm.queue.joinParty.send({ partyCode: created.partyCode, playerId: guestPlayerId });
		const joinRes = await waitFor(() =>
			mm.getJoinByPlayer({ partyCode: created.partyCode, playerId: guestPlayerId }),
		);

		const host = client.partyMatch
			.getOrCreate([created.matchId], { params: { playerToken: created.hostPlayerToken } })
			.connect();
		const guest = client.partyMatch
			.getOrCreate([created.matchId], { params: { playerToken: joinRes.playerToken } })
			.connect();

		const started = await host.start();
		expect(started.phase).toBe("in_progress");

		const room = await waitFor(async () => {
			const party = await mm.getParty({ partyCode: created.partyCode });
			return party?.status === "in_progress" ? party : null;
		});
		expect(room.status).toBe("in_progress");

		const finished = await host.finish();
		expect(finished.phase).toBe("finished");

		await Promise.all([host.dispose(), guest.dispose(), mm.dispose()]);
	}, 15_000);

	test("async turn-based invite + open pool without tick loop", async (ctx) => {
		const { client } = await setupTest(ctx, registry);
		const mm = client.asyncTurnBasedMatchmaker.getOrCreate(["main"]).connect();

		const inviteCode = `invite-${Date.now()}`;
		await mm.queue.createInvite.send({ inviteCode, fromPlayerId: "turn-a", toPlayerId: "turn-b" });
		await mm.queue.acceptInvite.send({ inviteCode, playerId: "turn-b" });
		const accepted = await waitFor(() => mm.getAssignment({ playerId: "turn-b" }));
		const turnAAssignment = await waitFor(() => mm.getAssignment({ playerId: "turn-a" }));

		const a = client.asyncTurnBasedMatch
			.getOrCreate([accepted.matchId], { params: { playerToken: turnAAssignment.playerToken } })
			.connect();
		const b = client.asyncTurnBasedMatch
			.getOrCreate([accepted.matchId], { params: { playerToken: accepted.playerToken } })
			.connect();

		await expect(b.submitTurn({ move: "bad-first" })).rejects.toMatchObject({
			code: "not_your_turn",
		});

		await a.submitTurn({ move: "open" });
		await b.submitTurn({ move: "reply" });
		const finished = await a.finish({ winnerPlayerId: "turn-a" });
		expect(finished.phase).toBe("finished");

		await mm.queue.joinOpenPool.send({ playerId: "pool-a" });
		await mm.queue.joinOpenPool.send({ playerId: "pool-b" });

		const assignedPoolA = await waitFor(() => mm.getAssignment({ playerId: "pool-a" }));
		const assignedPoolB = await waitFor(() => mm.getAssignment({ playerId: "pool-b" }));
		expect(assignedPoolA.matchId).toBe(assignedPoolB.matchId);

		await Promise.all([a.dispose(), b.dispose(), mm.dispose()]);
	}, 15_000);

	test("ranked elo matchmaking + 20 tps", async (ctx) => {
		const { client } = await setupTest(ctx, registry);
		const mm = client.rankedMatchmaker.getOrCreate(["main"]).connect();

		await mm.queue.debugSetRating.send({ playerId: "rank-a", elo: 1200 });
		await mm.queue.debugSetRating.send({ playerId: "rank-b", elo: 1210 });
		await waitUntil(async () => (await mm.getRating({ playerId: "rank-a" })).elo === 1200);
		await waitUntil(async () => (await mm.getRating({ playerId: "rank-b" })).elo === 1210);

		await mm.queue.queueForMatch.send({ playerId: "rank-a" });
		await mm.queue.queueForMatch.send({ playerId: "rank-b" });

		const assignment = await waitFor(() => mm.getAssignment({ playerId: "rank-a" }));
		if (!assignment) throw new Error("missing ranked assignment");
		const matchId = assignment.matchId;
		expect(matchId).toMatch(/^ranked-/);
		const rankB = await waitFor(() => mm.getAssignment({ playerId: "rank-b" }));

		const a = client.rankedMatch
			.getOrCreate([matchId], { params: { playerToken: assignment.playerToken } })
			.connect();
		const b = client.rankedMatch
			.getOrCreate([matchId], { params: { playerToken: rankB.playerToken } })
			.connect();

		await sleep(120);
		const live = await a.getSnapshot();
		expect(live.phase).toBe("live");
		expect(live.tick).toBeGreaterThanOrEqual(2);

		await a.finish({ winnerPlayerId: "rank-a" });
		await waitUntil(async () => (await mm.getRating({ playerId: "rank-a" })).elo > 1200);
		await waitUntil(async () => (await mm.getRating({ playerId: "rank-b" })).elo < 1210);
		const ra = await mm.getRating({ playerId: "rank-a" });
		const rb = await mm.getRating({ playerId: "rank-b" });
		expect(ra.elo).toBeGreaterThan(1200);
		expect(rb.elo).toBeLessThan(1210);

		await Promise.all([a.dispose(), b.dispose(), mm.dispose()]);
	}, 15_000);

	test("battle royale queue + 10 tps loop", async (ctx) => {
		const { client } = await setupTest(ctx, registry);
		const mm = client.battleRoyaleMatchmaker.getOrCreate(["main"]).connect();

		await mm.queue.joinQueue.send({ playerId: "br-a" });
		await mm.queue.joinQueue.send({ playerId: "br-b" });
		await mm.queue.joinQueue.send({ playerId: "br-c" });

		const assignment = await waitFor(() => mm.getAssignment({ playerId: "br-a" }));
		if (!assignment) throw new Error("missing battle royale assignment");
		const matchId = assignment.matchId;
		const assignmentB = await waitFor(() => mm.getAssignment({ playerId: "br-b" }));
		const assignmentC = await waitFor(() => mm.getAssignment({ playerId: "br-c" }));
		if (!assignmentB || !assignmentC) throw new Error("missing battle royale assignments");

		const a = client.battleRoyaleMatch
			.getOrCreate([matchId], { params: { playerToken: assignment.playerToken } })
			.connect();
		const b = client.battleRoyaleMatch
			.getOrCreate([matchId], { params: { playerToken: assignmentB.playerToken } })
			.connect();
		const cConn = client.battleRoyaleMatch
			.getOrCreate([matchId], { params: { playerToken: assignmentC.playerToken } })
			.connect();
		await a.startNow();

		await sleep(220);
		const live = await a.getSnapshot();
		expect(live.phase).toBe("active");
		expect(live.tick).toBeGreaterThanOrEqual(2);
		expect(live.zoneRadius).toBeLessThan(120);

		await a.eliminate({ victimPlayerId: "br-b" });
		await a.eliminate({ victimPlayerId: "br-c" });
		const finished = await a.getSnapshot();
		expect(finished.phase).toBe("finished");
		expect(finished.winnerPlayerId).toBe("br-a");

		await waitUntil(async () => (await mm.getAssignment({ playerId: "br-a" })) == null);

		await Promise.all([a.dispose(), b.dispose(), cConn.dispose(), mm.dispose()]);
	}, 15_000);

	test("open world chunking with world index + chunk actors", async (ctx) => {
		const { client } = await setupTest(ctx, registry);

		const index = client.openWorldIndex.getOrCreate(["main"]).connect();
		const world = await index.registerWorld({ worldId: "world-a", chunkSize: 128 });
		expect(world.chunkSize).toBe(128);

		const windowRes = await index.listChunkWindow({
			worldId: "world-a",
			centerWorldX: 32,
			centerWorldY: 48,
			radius: 1,
		});
		expect(windowRes.chunks.length).toBe(9);
		expect(windowRes.centerChunkX).toBe(0);
		expect(windowRes.centerChunkY).toBe(0);

		const resolved = await index.resolveChunk({
			worldId: "world-a",
			worldX: 64,
			worldY: 96,
		});
		expect(resolved.chunkX).toBe(0);
		expect(resolved.chunkY).toBe(0);

		const chunk = client.openWorldChunk.getOrCreate(resolved.chunkKey).connect();
		await chunk.join({
			playerId: "ow-a",
			name: "alice",
			worldX: 64,
			worldY: 96,
		});
		const joined = await chunk.getSnapshot();
		expect(joined.playerCount).toBe(1);
		expect(joined.chunkX).toBe(0);

		const stay = await chunk.move({
			playerId: "ow-a",
			worldX: 110,
			worldY: 120,
		});
		expect(stay.moved).toBe(true);

		const handoffX = 300;
		const expectedNextChunkX = Math.floor(handoffX / joined.chunkSize);
		const cross = await chunk.move({
			playerId: "ow-a",
			worldX: handoffX,
			worldY: 120,
		});
		expect(cross.moved).toBe(false);
		if (cross.moved) throw new Error("expected cross-chunk handoff");
		expect(cross.reason).toBe("cross_chunk");
		expect(cross.nextChunkKey[1]).toBe(String(expectedNextChunkX));
		expect(cross.nextChunkKey[2]).toBe("0");

		const nextChunk = client.openWorldChunk.getOrCreate(cross.nextChunkKey).connect();
		await nextChunk.join({
			playerId: "ow-a",
			name: "alice",
			worldX: handoffX,
			worldY: 120,
		});
		const nextSnapshot = await nextChunk.getSnapshot();
		expect(nextSnapshot.chunkX).toBe(expectedNextChunkX);
		expect(nextSnapshot.chunkY).toBe(0);
		expect(nextSnapshot.playerCount).toBe(1);

		await Promise.all([nextChunk.dispose(), chunk.dispose(), index.dispose()]);
	}, 15_000);
});

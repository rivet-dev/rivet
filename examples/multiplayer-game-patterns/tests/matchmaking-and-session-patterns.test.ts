import { setupTest } from "rivetkit/test";
import { describe, expect, test } from "vitest";
import { registry } from "../src/actors/index.ts";
import { INTERNAL_TOKEN } from "../src/auth.ts";

const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));
const expectForbidden = async (promise: Promise<unknown>) => {
	await expect(promise).rejects.toMatchObject({ code: "forbidden" });
};

describe("matchmaking and session patterns", () => {
	test("io-style open lobby + 10 tps match with movement", async (ctx) => {
		const { client } = await setupTest(ctx, registry);

		const mm = client.ioStyleMatchmaker.getOrCreate(["main"]).connect();
		const firstQueueResult = await mm.send(
			"findLobby",
			{},
			{ wait: true, timeout: 1_000 },
		);
		const secondQueueResult = await mm.send(
			"findLobby",
			{},
			{ wait: true, timeout: 1_000 },
		);
		expect(firstQueueResult?.status).toBe("completed");
		expect(secondQueueResult?.status).toBe("completed");
		const firstResponse = (firstQueueResult as { response?: { matchId?: string; playerId?: string; playerToken?: string } })?.response;
		const secondResponse = (secondQueueResult as { response?: { matchId?: string; playerId?: string; playerToken?: string } })?.response;
		expect(firstResponse?.playerId).toBeTypeOf("string");
		expect(firstResponse?.playerToken).toBeTypeOf("string");
		expect(secondResponse?.playerId).toBeTypeOf("string");
		expect(secondResponse?.playerToken).toBeTypeOf("string");
		expect(secondResponse?.matchId).toBe(firstResponse?.matchId);

		const a = client.ioStyleMatch
			.getOrCreate([firstResponse!.matchId!], { params: { playerToken: firstResponse!.playerToken! } })
			.connect();
		const b = client.ioStyleMatch
			.getOrCreate([firstResponse!.matchId!], { params: { playerToken: secondResponse!.playerToken! } })
			.connect();
		await sleep(260);

		const snapshot = await a.getSnapshot();
		expect(snapshot.playerCount).toBe(2);
		expect(snapshot.tick).toBeGreaterThanOrEqual(2);
		expect(Object.keys(snapshot.players)).toHaveLength(2);
		expect(snapshot.worldSize).toBe(600);

		// Record initial position and send input to move right.
		const initialX = snapshot.players[firstResponse!.playerId!]!.x;
		await a.setInput({ inputX: 1, inputY: 0 });
		await sleep(350);

		const moved = await a.getSnapshot();
		expect(moved.players[firstResponse!.playerId!]!.x).toBeGreaterThan(initialX);

		await Promise.all([a.dispose(), b.dispose(), mm.dispose()]);
	}, 15_000);

	test("arena queue-fill matchmaking with hybrid movement and hitscan combat", async (ctx) => {
		const { client } = await setupTest(ctx, registry);

		interface Assignment {
			matchId: string;
			playerId: string;
			playerToken: string;
			teamId: number;
			mode: string;
		}

		// Queue all 4 players concurrently. Each send returns immediately with a playerId.
		const mm = client.arenaMatchmaker.getOrCreate(["main"]).connect();
		const results = await Promise.all(
			Array.from({ length: 4 }, () =>
				mm.send("queueForMatch", { mode: "duo" }, { wait: true, timeout: 10_000 }),
			),
		);

		const queueEntries = results.map((r) => {
			const response = (r as {
				response?: { playerId: string; registrationToken: string };
			})?.response;
			expect(response?.playerId).toBeTypeOf("string");
			expect(response?.registrationToken).toBeTypeOf("string");
			return {
				playerId: response!.playerId,
				registrationToken: response!.registrationToken,
			};
		});
		await Promise.all(queueEntries.map((entry) => mm.registerPlayer(entry)));

		// Poll for assignments. The match should already be filled since we queued 4 players.
		const assignments: Assignment[] = [];
		for (const entry of queueEntries) {
			let assignment: Assignment | null = null;
			for (let i = 0; i < 50 && !assignment; i++) {
				assignment = await mm.getAssignment(entry) as Assignment | null;
				if (!assignment) await sleep(100);
			}
			expect(assignment).not.toBeNull();
			expect(assignment!.matchId).toBeTypeOf("string");
			expect(assignment!.playerToken).toBeTypeOf("string");
			assignments.push(assignment!);
		}

		const matchId = assignments[0]!.matchId;
		expect(assignments.every((a) => a.matchId === matchId)).toBe(true);

		// Connect all players to the match.
		const conns = assignments.map((a) =>
			client.arenaMatch
				.get([a.matchId], { params: { playerToken: a.playerToken } })
				.connect(),
		);
		await sleep(200);

		// All connected → phase should be live.
		const player0Id = assignments[0]!.playerId;
		const snap1 = await conns[0]!.getSnapshot();
		expect(snap1.phase).toBe("live");
		expect(Object.keys(snap1.players)).toHaveLength(4);
		expect(snap1.worldSize).toBe(600);

		// Test updatePosition on the first player.
		const initialX = snap1.players[player0Id]!.x;
		const initialY = snap1.players[player0Id]!.y;
		await conns[0]!.updatePosition({ x: initialX + 10, y: initialY });
		await sleep(100);
		const snap2 = await conns[0]!.getSnapshot();
		expect(snap2.players[player0Id]!.x).toBeCloseTo(initialX + 10, 0);

		// Test shoot: player 0 (team 0) shoots toward player 1 (team 1, different team).
		// Move them close together with multiple small steps.
		const targetPlayerId = assignments[1]!.playerId;
		for (let step = 0; step < 30; step++) {
			const snapStep = await conns[0]!.getSnapshot();
			const me = snapStep.players[player0Id]!;
			const target = snapStep.players[targetPlayerId]!;
			const dx = target.x - me.x;
			const dy = target.y - me.y;
			const dist = Math.sqrt(dx * dx + dy * dy);
			if (dist < 50) break;
			// Move toward target (speed-limited by server).
			const moveBy = Math.min(dist - 30, 40);
			await conns[0]!.updatePosition({
				x: me.x + (dx / dist) * moveBy,
				y: me.y + (dy / dist) * moveBy,
			});
			await sleep(60);
		}

		// Get positions, then shoot toward the target.
		let scored = false;
		for (let attempt = 0; attempt < 6 && !scored; attempt++) {
			const snapBeforeShoot = await conns[0]!.getSnapshot();
			const mePos = snapBeforeShoot.players[player0Id]!;
			const targetPos = snapBeforeShoot.players[targetPlayerId]!;
			const dx = targetPos.x - mePos.x;
			const dy = targetPos.y - mePos.y;
			const mag = Math.sqrt(dx * dx + dy * dy);
			if (mag === 0) continue;
			await conns[0]!.shoot({ dirX: dx / mag, dirY: dy / mag });
			await sleep(100);
			const afterShoot = await conns[0]!.getSnapshot();
			scored = afterShoot.players[player0Id]!.score >= 1;
		}
		const snap3 = await conns[0]!.getSnapshot();
		expect(snap3.phase).toBe("live");
		expect(snap3.players[player0Id]).toBeDefined();

		await Promise.all([...conns.map((c) => c.dispose()), mm.dispose()]);
	}, 15_000);

	test("party lobby with host controls and member management", async (ctx) => {
		const { client } = await setupTest(ctx, registry);

		// Create a party.
		const mm = client.partyMatchmaker.getOrCreate(["main"]).connect();
		const createResult = await mm.send(
			"createParty",
			{ hostName: "Host" },
			{ wait: true, timeout: 5_000 },
		);
		const createResponse = (createResult as { response?: { matchId: string; playerId: string; playerToken: string; partyCode: string } })?.response;
		expect(createResponse?.matchId).toBeTypeOf("string");
		expect(createResponse?.partyCode).toHaveLength(6);

		// Host connects.
		const hostConn = client.partyMatch
			.get([createResponse!.matchId], { params: { playerToken: createResponse!.playerToken } })
			.connect();
		await sleep(200);

		const snap1 = await hostConn.getSnapshot();
		expect(snap1.partyCode).toBe(createResponse!.partyCode);
		expect(snap1.phase).toBe("waiting");
		expect(Object.keys(snap1.members)).toHaveLength(1);
		const hostMember = snap1.members[createResponse!.playerId];
		expect(hostMember.isHost).toBe(true);

		// Second player joins by code.
		const joinResult = await mm.send(
			"joinParty",
			{ partyCode: createResponse!.partyCode, playerName: "Player2" },
			{ wait: true, timeout: 5_000 },
		);
		const joinResponse = (joinResult as { response?: { matchId: string; playerId: string; playerToken: string } })?.response;
		expect(joinResponse?.matchId).toBe(createResponse!.matchId);

		const p2Conn = client.partyMatch
			.get([joinResponse!.matchId], { params: { playerToken: joinResponse!.playerToken } })
			.connect();
		await sleep(200);

		const snap2 = await hostConn.getSnapshot();
		expect(Object.keys(snap2.members)).toHaveLength(2);

		// Toggle ready and start game.
		await p2Conn.toggleReady();
		await sleep(100);
		await hostConn.startGame();
		await sleep(100);

		const snap3 = await hostConn.getSnapshot();
		expect(snap3.phase).toBe("playing");

		// Finish game.
		await hostConn.finishGame();
		await sleep(100);

		const snap4 = await hostConn.getSnapshot();
		expect(snap4.phase).toBe("finished");

		await Promise.all([hostConn.dispose(), p2Conn.dispose(), mm.dispose()]);
	}, 15_000);

	test("turn-based tic-tac-toe with moves and win detection", async (ctx) => {
		const { client } = await setupTest(ctx, registry);

		const mm = client.turnBasedMatchmaker.getOrCreate(["main"]).connect();

		// Player X creates a game.
		const createResult = await mm.send(
			"createGame",
			{ playerName: "PlayerX" },
			{ wait: true, timeout: 5_000 },
		);
		const createResponse = (createResult as { response?: { matchId: string; playerId: string; playerToken: string; inviteCode: string } })?.response;
		expect(createResponse?.matchId).toBeTypeOf("string");
		expect(createResponse?.inviteCode).toHaveLength(6);

		// Player O joins by code.
		const joinResult = await mm.send(
			"joinByCode",
			{ inviteCode: createResponse!.inviteCode, playerName: "PlayerO" },
			{ wait: true, timeout: 5_000 },
		);
		const joinResponse = (joinResult as { response?: { matchId: string; playerId: string; playerToken: string } })?.response;
		expect(joinResponse?.matchId).toBe(createResponse!.matchId);

		// Both connect.
		const xConn = client.turnBasedMatch
			.get([createResponse!.matchId], { params: { playerToken: createResponse!.playerToken } })
			.connect();
		const oConn = client.turnBasedMatch
			.get([joinResponse!.matchId], { params: { playerToken: joinResponse!.playerToken } })
			.connect();
		await sleep(200);

		const snap1 = await xConn.getSnapshot();
		expect(snap1.currentTurn).toBe("X");
		expect(Object.keys(snap1.players)).toHaveLength(2);

		// Play a quick game: X wins with top row.
		await xConn.makeMove({ row: 0, col: 0 }); // X
		await sleep(50);
		await oConn.makeMove({ row: 1, col: 0 }); // O
		await sleep(50);
		await xConn.makeMove({ row: 0, col: 1 }); // X
		await sleep(50);
		await oConn.makeMove({ row: 1, col: 1 }); // O
		await sleep(50);
		await xConn.makeMove({ row: 0, col: 2 }); // X wins
		await sleep(100);

		const snap2 = await xConn.getSnapshot();
		expect(snap2.result).toBe("x_wins");
		expect(snap2.board[0]).toEqual(["X", "X", "X"]);

		await Promise.all([xConn.dispose(), oConn.dispose(), mm.dispose()]);
	}, 15_000);

	test("ranked 1v1 matchmaking with ELO pairing", async (ctx) => {
		const { client } = await setupTest(ctx, registry);

		interface RankedAssignment {
			matchId: string;
			username: string;
			rating: number;
			playerToken: string;
		}

		const usernames = ["TestPlayer1", "TestPlayer2"];

		// Queue both players concurrently.
		const mm = client.rankedMatchmaker.getOrCreate(["main"]).connect();
		const queueResults = await Promise.all(
			usernames.map((username) =>
				mm.send("queueForMatch", { username }, { wait: true, timeout: 10_000 }),
			),
		);
		const registrationTokenByUsername = new Map<string, string>();
		for (const [idx, username] of usernames.entries()) {
			const response = (queueResults[idx] as {
				response?: { registrationToken: string };
			})?.response;
			expect(response?.registrationToken).toBeTypeOf("string");
			registrationTokenByUsername.set(username, response!.registrationToken);
		}
		await Promise.all(
			usernames.map((username) =>
				mm.registerPlayer({
					username,
					registrationToken: registrationTokenByUsername.get(username)!,
				}),
			),
		);

		// Poll for assignments.
		const assignments: RankedAssignment[] = [];
		for (const username of usernames) {
			let assignment: RankedAssignment | null = null;
			for (let i = 0; i < 50 && !assignment; i++) {
				assignment = await mm.getAssignment({
					username,
					registrationToken: registrationTokenByUsername.get(username)!,
				}) as RankedAssignment | null;
				if (!assignment) await sleep(100);
			}
			expect(assignment).not.toBeNull();
			expect(assignment!.matchId).toBeTypeOf("string");
			expect(assignment!.username).toBeTypeOf("string");
			assignments.push(assignment!);
		}

		// Connect both players to the match.
		const conns = assignments.map((a) =>
			client.rankedMatch
				.get([a.matchId], { params: { playerToken: a.playerToken } })
				.connect(),
		);
		await sleep(200);

		const snap = await conns[0]!.getSnapshot();
		expect(snap.phase).toBe("live");
		expect(Object.keys(snap.players)).toHaveLength(2);
		expect(snap.scoreLimit).toBe(5);

		await Promise.all([...conns.map((c) => c.dispose()), mm.dispose()]);
	}, 15_000);

	test("battle-royale lobby matchmaking and snapshot", async (ctx) => {
		const { client } = await setupTest(ctx, registry);

		const mm = client.battleRoyaleMatchmaker.getOrCreate(["main"]).connect();
		const result1 = await mm.send("findMatch", {}, { wait: true, timeout: 5_000 });
		const result2 = await mm.send("findMatch", {}, { wait: true, timeout: 5_000 });
		const r1 = (result1 as { response?: { matchId: string; playerId: string; playerToken: string } })?.response;
		const r2 = (result2 as { response?: { matchId: string; playerId: string; playerToken: string } })?.response;
		expect(r1?.matchId).toBeTypeOf("string");
		expect(r2?.matchId).toBe(r1?.matchId);

		const a = client.battleRoyaleMatch
			.get([r1!.matchId], { params: { playerToken: r1!.playerToken } })
			.connect();
		const b = client.battleRoyaleMatch
			.get([r2!.matchId], { params: { playerToken: r2!.playerToken } })
			.connect();
		await sleep(300);

		const snap = await a.getSnapshot();
		expect(snap.phase).toBe("lobby");
		expect(snap.playerCount).toBe(2);
		expect(snap.worldSize).toBe(1200);
		expect(snap.capacity).toBe(16);
		expect(Object.keys(snap.players)).toHaveLength(2);

		await Promise.all([a.dispose(), b.dispose(), mm.dispose()]);
	}, 15_000);

	test("open-world chunk routing and movement", async (ctx) => {
		const { client } = await setupTest(ctx, registry);

		const index = client.openWorldIndex.getOrCreate(["main"]).connect();
		const result = await index.send(
			"getChunkForPosition",
			{ x: 600, y: 600, playerName: "Explorer" },
			{ wait: true, timeout: 5_000 },
		);
		const response = (result as { response?: { chunkKey: [string, number, number]; playerId: string; playerToken: string } })?.response;
		expect(response?.chunkKey).toEqual(["default", 0, 0]);
		expect(response?.playerId).toBeTypeOf("string");
		expect(response?.playerToken).toBeTypeOf("string");

		const chunk = client.openWorldChunk
			.getOrCreate(["default", "0", "0"], { params: { playerToken: response!.playerToken } })
			.connect();
		await sleep(300);

		const snap = await chunk.getSnapshot();
		expect(snap.chunkX).toBe(0);
		expect(snap.chunkY).toBe(0);
		expect(snap.chunkSize).toBe(1200);
		expect(Object.keys(snap.players)).toHaveLength(1);

		// Send movement input.
		await chunk.setInput({ inputX: 1, inputY: 0 });
		await sleep(350);

		const snap2 = await chunk.getSnapshot();
		const myPos = snap2.players[response!.playerId];
		expect(myPos).toBeDefined();
		expect(myPos!.x).toBeGreaterThan(600);

		await Promise.all([chunk.dispose(), index.dispose()]);
	}, 15_000);

	test("open-world chunk transfer moves player to new chunk", async (ctx) => {
		const { client } = await setupTest(ctx, registry);

		// Spawn player near the right edge of chunk 0,0.
		const index = client.openWorldIndex.getOrCreate(["main"]).connect();
		const result = await index.send(
			"getChunkForPosition",
			{ x: 1190, y: 600, playerName: "Traveler" },
			{ wait: true, timeout: 5_000 },
		);
		const r1 = (result as { response?: { chunkKey: [string, number, number]; playerId: string; playerToken: string } })?.response;
		expect(r1?.chunkKey).toEqual(["default", 0, 0]);

		// Connect and move right until clamped at boundary.
		const chunk0 = client.openWorldChunk
			.getOrCreate(["default", "0", "0"], { params: { playerToken: r1!.playerToken } })
			.connect();
		await sleep(200);

		await chunk0.setInput({ inputX: 1, inputY: 0 });
		await sleep(500);

		const snapAtEdge = await chunk0.getSnapshot();
		const posAtEdge = snapAtEdge.players[r1!.playerId];
		expect(posAtEdge).toBeDefined();
		expect(posAtEdge!.x).toBe(1199); // Clamped to CHUNK_SIZE - 1.

		// Now simulate what the client does: request transfer to the next chunk.
		const absX = 0 * 1200 + posAtEdge!.x + 1; // One pixel into next chunk.
		const absY = 0 * 1200 + posAtEdge!.y;
		const transferResult = await index.send(
			"getChunkForPosition",
			{ x: absX, y: absY, playerName: "Traveler" },
			{ wait: true, timeout: 5_000 },
		);
		const r2 = (transferResult as { response?: { chunkKey: [string, number, number]; playerId: string; playerToken: string } })?.response;
		expect(r2?.chunkKey).toEqual(["default", 1, 0]);
		expect(r2?.playerId).toBeTypeOf("string");
		expect(r2?.playerToken).toBeTypeOf("string");

		// Connect to new chunk and verify player exists there.
		const chunk1 = client.openWorldChunk
			.getOrCreate(["default", "1", "0"], { params: { playerToken: r2!.playerToken } })
			.connect();
		await sleep(300);

		const snapNewChunk = await chunk1.getSnapshot();
		expect(snapNewChunk.chunkX).toBe(1);
		expect(snapNewChunk.chunkY).toBe(0);
		const newPos = snapNewChunk.players[r2!.playerId];
		expect(newPos).toBeDefined();
		// Player should be at x=0 (just crossed boundary), not at center.
		expect(newPos!.x).toBeLessThan(100);

		// Player should be able to move in the new chunk.
		await chunk1.setInput({ inputX: 1, inputY: 0 });
		await sleep(350);
		const snapMoved = await chunk1.getSnapshot();
		expect(snapMoved.players[r2!.playerId]!.x).toBeGreaterThan(newPos!.x);

		await Promise.all([chunk0.dispose(), chunk1.dispose(), index.dispose()]);
	}, 15_000);

	test("idle building and production with leaderboard", async (ctx) => {
		const { client } = await setupTest(ctx, registry);

		const playerId = crypto.randomUUID();
		const world = client.idleWorld.getOrCreate([playerId]).connect();

		// Initialize.
		await world.initialize({ playerName: "Builder" });
		const state1 = await world.getState();
		expect(state1.playerName).toBe("Builder");
		expect(state1.resources).toBe(10);
		expect(state1.buildings).toHaveLength(1);
		expect(state1.buildings[0]!.typeId).toBe("farm");

		// Check leaderboard actor.
		const lb = client.idleLeaderboard.getOrCreate(["main"]).connect();
		await sleep(100);
		const scores = await lb.getTopScores({ limit: 10 });
		expect(Array.isArray(scores)).toBe(true);

		await Promise.all([world.dispose(), lb.dispose()]);
	}, 15_000);

	test("forbidden access control paths are blocked across matchmaking patterns", async (ctx) => {
		const { client } = await setupTest(ctx, registry);

		await expectForbidden(
			client.ioStyleMatchmaker
				.getOrCreate(["main"])
				.send("updateMatch", { matchId: "nope", playerCount: 1 }),
		);
		await expectForbidden(
			client.arenaMatchmaker
				.getOrCreate(["main"])
				.send("matchCompleted", { matchId: "nope" }),
		);
		await expectForbidden(
			client.partyMatchmaker
				.getOrCreate(["main"])
				.send("closeParty", { matchId: "nope" }),
		);
		await expectForbidden(
			client.turnBasedMatchmaker
				.getOrCreate(["main"])
				.send("closeMatch", { matchId: "nope" }),
		);
		await expectForbidden(
			client.rankedMatchmaker
				.getOrCreate(["main"])
				.send("matchCompleted", {
					matchId: "nope",
					winnerUsername: "a",
					loserUsername: "b",
					winnerNewRating: 1200,
					loserNewRating: 1200,
				}),
		);
		await expectForbidden(
			client.battleRoyaleMatchmaker
				.getOrCreate(["main"])
				.send("updateMatch", {
					matchId: "nope",
					playerCount: 2,
					isStarted: false,
				}),
		);

		const arenaMm = client.arenaMatchmaker.getOrCreate(["main"]).connect();
		const arenaQueueResult = await arenaMm.send(
			"queueForMatch",
			{ mode: "1v1" },
			{ wait: true, timeout: 5_000 },
		);
		const arenaQueueResponse = (arenaQueueResult as {
			response?: { playerId: string; registrationToken: string };
		})?.response;
		expect(arenaQueueResponse?.playerId).toBeTypeOf("string");
		expect(arenaQueueResponse?.registrationToken).toBeTypeOf("string");
		await expectForbidden(
			arenaMm.registerPlayer({
				playerId: arenaQueueResponse!.playerId,
				registrationToken: "wrong-registration-token",
			}),
		);
		await arenaMm.dispose();

		const rankedMm = client.rankedMatchmaker.getOrCreate(["main"]).connect();
		const rankedQueueResult = await rankedMm.send(
			"queueForMatch",
			{ username: `Forbidden#${crypto.randomUUID()}` },
			{ wait: true, timeout: 5_000 },
		);
		const rankedQueueResponse = (rankedQueueResult as {
			response?: { registrationToken: string };
		})?.response;
		expect(rankedQueueResponse?.registrationToken).toBeTypeOf("string");
		await expectForbidden(
			rankedMm.registerPlayer({
				username: `Forbidden#${crypto.randomUUID()}`,
				registrationToken: rankedQueueResponse!.registrationToken,
			}),
		);
		await rankedMm.dispose();

		const ioMatchId = crypto.randomUUID();
		await client.ioStyleMatch.create([ioMatchId], { input: { matchId: ioMatchId } });
		const ioPlayerToken = crypto.randomUUID();
		await client.ioStyleMatch
			.get([ioMatchId], { params: { internalToken: INTERNAL_TOKEN } })
			.createPlayer({ playerId: "io-player", playerToken: ioPlayerToken });
		await expectForbidden(
			client.ioStyleMatch
				.get([ioMatchId], { params: { playerToken: ioPlayerToken } })
				.createPlayer({ playerId: "other", playerToken: crypto.randomUUID() }),
		);

		const partyMatchId = crypto.randomUUID();
		await client.partyMatch.create([partyMatchId], {
			input: { matchId: partyMatchId, partyCode: "ABC123" },
		});
		const partyPlayerToken = crypto.randomUUID();
		await client.partyMatch
			.get([partyMatchId], { params: { internalToken: INTERNAL_TOKEN } })
			.createPlayer({
				playerId: "party-player",
				playerToken: partyPlayerToken,
				playerName: "Party Player",
				isHost: true,
			});
		await expectForbidden(
			client.partyMatch
				.get([partyMatchId], { params: { playerToken: partyPlayerToken } })
				.createPlayer({
					playerId: "intruder",
					playerToken: crypto.randomUUID(),
					playerName: "Intruder",
					isHost: false,
				}),
		);

		const turnMatchId = crypto.randomUUID();
		await client.turnBasedMatch.create([turnMatchId], {
			input: { matchId: turnMatchId },
		});
		const turnPlayerToken = crypto.randomUUID();
		await client.turnBasedMatch
			.get([turnMatchId], { params: { internalToken: INTERNAL_TOKEN } })
			.createPlayer({
				playerId: "turn-player",
				playerToken: turnPlayerToken,
				playerName: "Turn Player",
				symbol: "X",
			});
		await expectForbidden(
			client.turnBasedMatch
				.get([turnMatchId], { params: { playerToken: turnPlayerToken } })
				.createPlayer({
					playerId: "intruder",
					playerToken: crypto.randomUUID(),
					playerName: "Intruder",
					symbol: "O",
				}),
		);

		const observerChunk = client.openWorldChunk
			.getOrCreate(["default", "0", "0"], { params: { observer: "true" } })
			.connect();
		await expectForbidden(
			observerChunk.initialize({ worldId: "default", chunkX: 0, chunkY: 0 }),
		);
		await expectForbidden(observerChunk.placeBlock({ gridX: 0, gridY: 0 }));
		await observerChunk.dispose();

		await expectForbidden(
			client.rankedPlayer
				.getOrCreate(["forbidden-player"])
				.applyMatchResult({ won: true, newRating: 1300 }),
		);
		await expectForbidden(
			client.rankedLeaderboard
				.getOrCreate(["main"])
				.updatePlayer({ username: "forbidden", rating: 1200, wins: 1, losses: 0 }),
		);
		await expectForbidden(
			client.idleLeaderboard
				.getOrCreate(["main"])
				.updateScore({ playerId: "x", playerName: "x", totalProduced: 1 }),
		);

		const idleWorld = client.idleWorld.getOrCreate([crypto.randomUUID()]).connect();
		await idleWorld.initialize({ playerName: "Builder" });
		const idleState = await idleWorld.getState();
		await expectForbidden(
			idleWorld.collectProduction({ buildingId: idleState.buildings[0]!.id }),
		);
		await idleWorld.dispose();
	}, 15_000);
});

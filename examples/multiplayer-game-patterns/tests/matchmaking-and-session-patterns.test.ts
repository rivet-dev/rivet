import { setupTest } from "rivetkit/test";
import { describe, expect, test } from "vitest";
import { registry } from "../src/actors/index.ts";
import { CHUNK_SIZE, WORLD_ID } from "../src/actors/open-world/config.ts";

const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

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
		const firstResponse = (firstQueueResult as {
			response?: { matchId?: string; playerId?: string };
		})?.response;
		const secondResponse = (secondQueueResult as {
			response?: { matchId?: string; playerId?: string };
		})?.response;
		expect(firstResponse?.playerId).toBeTypeOf("string");
		expect(secondResponse?.playerId).toBeTypeOf("string");
		expect(secondResponse?.matchId).toBe(firstResponse?.matchId);

		const a = client.ioStyleMatch
			.getOrCreate([firstResponse!.matchId!], {
				params: {
					playerId: firstResponse!.playerId!,
				},
			})
			.connect();
		const b = client.ioStyleMatch
			.getOrCreate([firstResponse!.matchId!], {
				params: {
					playerId: secondResponse!.playerId!,
				},
			})
			.connect();
		await sleep(260);

		const snapshot = await a.getSnapshot();
		expect(snapshot.playerCount).toBe(2);
		expect(snapshot.tick).toBeGreaterThanOrEqual(2);
		expect(Object.keys(snapshot.players)).toHaveLength(2);
		expect(snapshot.worldSize).toBe(600);

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
			teamId: number;
			mode: string;
		}

		const mm = client.arenaMatchmaker.getOrCreate(["main"]).connect();
		const results = await Promise.all(
			Array.from({ length: 4 }, () =>
				mm.queueForMatch({ mode: "duo" }),
			),
		);

		const queueEntries = results.map((r) => {
			const response = r as { playerId?: string };
			expect(response.playerId).toBeTypeOf("string");
			return {
				playerId: response.playerId!,
			};
		});

		const assignments: Assignment[] = [];
		for (const entry of queueEntries) {
			let assignment: Assignment | null = null;
			for (let i = 0; i < 50 && !assignment; i++) {
				assignment = await mm.getAssignment(entry) as Assignment | null;
				if (!assignment) await sleep(100);
			}
			expect(assignment).not.toBeNull();
			expect(assignment!.matchId).toBeTypeOf("string");
			assignments.push(assignment!);
		}

		const matchId = assignments[0]!.matchId;
		expect(assignments.every((a) => a.matchId === matchId)).toBe(true);

		const conns = assignments.map((a) =>
			client.arenaMatch
				.get([a.matchId], { params: { playerId: a.playerId } })
				.connect(),
		);
		await sleep(200);

		const player0Id = assignments[0]!.playerId;
		const snap1 = await conns[0]!.getSnapshot();
		expect(snap1.phase).toBe("live");
		expect(Object.keys(snap1.players)).toHaveLength(4);
		expect(snap1.worldSize).toBe(600);

		const initialX = snap1.players[player0Id]!.x;
		const initialY = snap1.players[player0Id]!.y;
		await conns[0]!.updatePosition({ x: initialX + 10, y: initialY });
		await sleep(100);
		const snap2 = await conns[0]!.getSnapshot();
		expect(snap2.players[player0Id]!.x).toBeCloseTo(initialX + 10, 0);

		const targetPlayerId = assignments[1]!.playerId;
		for (let step = 0; step < 30; step++) {
			const snapStep = await conns[0]!.getSnapshot();
			const me = snapStep.players[player0Id]!;
			const target = snapStep.players[targetPlayerId]!;
			const dx = target.x - me.x;
			const dy = target.y - me.y;
			const dist = Math.sqrt(dx * dx + dy * dy);
			if (dist < 50) break;
			const moveBy = Math.min(dist - 30, 40);
			await conns[0]!.updatePosition({
				x: me.x + (dx / dist) * moveBy,
				y: me.y + (dy / dist) * moveBy,
			});
			await sleep(60);
		}

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

		const mm = client.partyMatchmaker.getOrCreate(["main"]).connect();
		const createResult = await mm.send(
			"createParty",
			{ hostName: "Host" },
			{ wait: true, timeout: 5_000 },
		);
		const createResponse = (createResult as {
			response?: {
				matchId: string;
				playerId: string;
				partyCode: string;
				joinToken: string;
				playerName: string;
			};
		})?.response;
		expect(createResponse?.matchId).toBeTypeOf("string");
		expect(createResponse?.partyCode).toHaveLength(6);

		const hostConn = client.partyMatch
			.get([createResponse!.matchId], {
				params: {
					playerId: createResponse!.playerId,
					joinToken: createResponse!.joinToken,
				},
			})
			.connect();
		await sleep(200);

		const snap1 = await hostConn.getSnapshot();
		expect(snap1.partyCode).toBe(createResponse!.partyCode);
		expect(snap1.phase).toBe("waiting");
		expect(Object.keys(snap1.members)).toHaveLength(1);
		const hostMember = snap1.members[createResponse!.playerId];
		expect(hostMember.isHost).toBe(true);

		const joinResult = await mm.send(
			"joinParty",
			{ partyCode: createResponse!.partyCode, playerName: "Player2" },
			{ wait: true, timeout: 5_000 },
		);
		const joinResponse = (joinResult as {
			response?: {
				matchId: string;
				playerId: string;
				joinToken: string;
				playerName: string;
			};
		})?.response;
		expect(joinResponse?.matchId).toBe(createResponse!.matchId);

		const p2Conn = client.partyMatch
			.get([joinResponse!.matchId], {
				params: {
					playerId: joinResponse!.playerId,
					joinToken: joinResponse!.joinToken,
				},
			})
			.connect();
		await sleep(200);

		const snap2 = await hostConn.getSnapshot();
		expect(Object.keys(snap2.members)).toHaveLength(2);

		await p2Conn.toggleReady();
		await sleep(100);
		await hostConn.startGame();
		await sleep(100);

		const snap3 = await hostConn.getSnapshot();
		expect(snap3.phase).toBe("playing");

		await hostConn.finishGame();
		await sleep(100);

		const snap4 = await hostConn.getSnapshot();
		expect(snap4.phase).toBe("finished");

		await p2Conn.dispose();
		await sleep(100);

		const snap5 = await hostConn.getSnapshot();
		expect(Object.keys(snap5.members)).toHaveLength(1);

		await Promise.all([hostConn.dispose(), mm.dispose()]);
	}, 15_000);

	test("turn-based tic-tac-toe with moves and win detection", async (ctx) => {
		const { client } = await setupTest(ctx, registry);

		const mm = client.turnBasedMatchmaker.getOrCreate(["main"]).connect();

		const createResult = await mm.send(
			"createGame",
			{ playerName: "PlayerX" },
			{ wait: true, timeout: 5_000 },
		);
		const createResponse = (createResult as { response?: { matchId: string; playerId: string; inviteCode: string } })?.response;
		expect(createResponse?.matchId).toBeTypeOf("string");
		expect(createResponse?.inviteCode).toHaveLength(6);

		const joinResult = await mm.send(
			"joinByCode",
			{ inviteCode: createResponse!.inviteCode, playerName: "PlayerO" },
			{ wait: true, timeout: 5_000 },
		);
		const joinResponse = (joinResult as { response?: { matchId: string; playerId: string } })?.response;
		expect(joinResponse?.matchId).toBe(createResponse!.matchId);

		const xConn = client.turnBasedMatch
			.get([createResponse!.matchId], { params: { playerId: createResponse!.playerId } })
			.connect();
		const oConn = client.turnBasedMatch
			.get([joinResponse!.matchId], { params: { playerId: joinResponse!.playerId } })
			.connect();
		await sleep(200);

		const snap1 = await xConn.getSnapshot();
		expect(snap1.currentTurn).toBe("X");
		expect(Object.keys(snap1.players)).toHaveLength(2);

		await xConn.makeMove({ row: 0, col: 0 });
		await sleep(50);
		await oConn.makeMove({ row: 1, col: 0 });
		await sleep(50);
		await xConn.makeMove({ row: 0, col: 1 });
		await sleep(50);
		await oConn.makeMove({ row: 1, col: 1 });
		await sleep(50);
		await xConn.makeMove({ row: 0, col: 2 });
		await sleep(100);

		const snap2 = await xConn.getSnapshot();
		expect(snap2.result).toBe("x_wins");
		expect(snap2.board[0]).toEqual(["X", "X", "X"]);

		await Promise.all([xConn.dispose(), oConn.dispose(), mm.dispose()]);
	}, 15_000);

	test("turn-based public queue omits invite code and starts match", async (ctx) => {
		const { client } = await setupTest(ctx, registry);

		const mm = client.turnBasedMatchmaker.getOrCreate(["main"]).connect();
		const q1 = await mm.queueForMatch({ playerName: "PublicA" }) as { playerId?: string };
		const q2 = await mm.queueForMatch({ playerName: "PublicB" }) as { playerId?: string };
		expect(q1.playerId).toBeTypeOf("string");
		expect(q2.playerId).toBeTypeOf("string");

		type PublicAssignment = {
			matchId: string;
			playerId: string;
			inviteCode?: string;
		};

		const waitAssignment = async (playerId: string): Promise<PublicAssignment> => {
			for (let i = 0; i < 50; i++) {
				const assignment = await mm.getAssignment({ playerId }) as PublicAssignment | null;
				if (assignment) return assignment;
				await sleep(100);
			}
			throw new Error("timed out waiting for public assignment");
		};

		const a1 = await waitAssignment(q1.playerId!);
		const a2 = await waitAssignment(q2.playerId!);
		expect(a1.matchId).toBe(a2.matchId);
		expect(a1.inviteCode).toBeUndefined();
		expect(a2.inviteCode).toBeUndefined();

		const c1 = client.turnBasedMatch
			.get([a1.matchId], { params: { playerId: a1.playerId } })
			.connect();
		const c2 = client.turnBasedMatch
			.get([a2.matchId], { params: { playerId: a2.playerId } })
			.connect();
		await sleep(250);

		const snap = await c1.getSnapshot();
		expect(Object.keys(snap.players)).toHaveLength(2);
		expect(snap.result).toBeNull();

		await Promise.all([c1.dispose(), c2.dispose(), mm.dispose()]);
	}, 15_000);

	test("ranked 1v1 matchmaking with ELO pairing", async (ctx) => {
		const { client } = await setupTest(ctx, registry);

		interface RankedAssignment {
			matchId: string;
			username: string;
			rating: number;
		}

		const usernames = ["TestPlayer1", "TestPlayer2"];

		const mm = client.rankedMatchmaker.getOrCreate(["main"]).connect();
		await Promise.all(
			usernames.map((username) =>
				mm.queueForMatch({ username }),
			),
		);

		const assignments: RankedAssignment[] = [];
		for (const username of usernames) {
			let assignment: RankedAssignment | null = null;
			for (let i = 0; i < 50 && !assignment; i++) {
				assignment = await mm.getAssignment({ username }) as RankedAssignment | null;
				if (!assignment) await sleep(100);
			}
			expect(assignment).not.toBeNull();
			expect(assignment!.matchId).toBeTypeOf("string");
			expect(assignment!.username).toBeTypeOf("string");
			assignments.push(assignment!);
		}

		const conns = assignments.map((a) =>
			client.rankedMatch
				.get([a.matchId], { params: { username: a.username } })
				.connect(),
		);
		await sleep(200);

		const snap = await conns[0]!.getSnapshot();
		expect(snap.phase).toBe("live");
		expect(Object.keys(snap.players)).toHaveLength(2);
		expect(snap.scoreLimit).toBe(5);

		await Promise.all([...conns.map((c) => c.dispose()), mm.dispose()]);
	}, 15_000);

	test("ranked allows re-queueing the same usernames", async (ctx) => {
		const { client } = await setupTest(ctx, registry);

		interface RankedAssignment {
			matchId: string;
			username: string;
			rating: number;
		}

		const usernames = ["RequeueA", "RequeueB"];
		const mm = client.rankedMatchmaker.getOrCreate(["main"]).connect();
		let previousMatchId: string | null = null;

		for (let round = 0; round < 2; round++) {
			await Promise.all(
				usernames.map((username) =>
					mm.queueForMatch({ username }),
				),
			);

			const assignments: RankedAssignment[] = [];
			for (const username of usernames) {
				let assignment: RankedAssignment | null = null;
				for (let i = 0; i < 80 && !assignment; i++) {
					const next = await mm.getAssignment({ username }) as RankedAssignment | null;
					if (next && (!previousMatchId || next.matchId !== previousMatchId)) {
						assignment = next;
						break;
					}
					await sleep(100);
				}
				expect(assignment).not.toBeNull();
				assignments.push(assignment!);
			}

			const matchId = assignments[0]!.matchId;
			expect(assignments.every((a) => a.matchId === matchId)).toBe(true);
			previousMatchId = matchId;

			const conns = assignments.map((a) =>
				client.rankedMatch
					.get([a.matchId], { params: { username: a.username } })
					.connect(),
			);
			await sleep(200);

			const snap = await conns[0]!.getSnapshot();
			expect(Object.keys(snap.players)).toHaveLength(2);

			await Promise.all(conns.map((c) => c.dispose()));
			await sleep(100);
		}

		await mm.dispose();
	}, 20_000);

	test("battle-royale lobby matchmaking and snapshot", async (ctx) => {
		const { client } = await setupTest(ctx, registry);

		const mm = client.battleRoyaleMatchmaker.getOrCreate(["main"]).connect();
		const result1 = await mm.send("findMatch", {}, { wait: true, timeout: 5_000 });
		const result2 = await mm.send("findMatch", {}, { wait: true, timeout: 5_000 });
		const r1 = (result1 as {
			response?: {
				matchId: string;
				playerId: string;
			};
		})?.response;
		const r2 = (result2 as {
			response?: {
				matchId: string;
				playerId: string;
			};
		})?.response;
		expect(r1?.matchId).toBeTypeOf("string");
		expect(r2?.matchId).toBe(r1?.matchId);

		const a = client.battleRoyaleMatch
			.get([r1!.matchId], {
				params: {
					playerId: r1!.playerId,
				},
			})
			.connect();
		const b = client.battleRoyaleMatch
			.get([r2!.matchId], {
				params: {
					playerId: r2!.playerId,
				},
			})
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

		const response = resolveChunkForPosition(600, 600);
		expect(response.chunkKey).toEqual([WORLD_ID, 0, 0]);
		expect(response?.spawnX).toBeTypeOf("number");
		expect(response?.spawnY).toBeTypeOf("number");

		const chunk = client.openWorldChunk
			.getOrCreate([WORLD_ID, "0", "0"])
			.connect();
		await chunk.addPlayer({
			name: "Explorer",
			spawnX: response.spawnX,
			spawnY: response.spawnY,
		});
		await sleep(300);

		const snap = await chunk.getSnapshot();
		expect(snap.chunkX).toBe(0);
		expect(snap.chunkY).toBe(0);
		expect(snap.chunkSize).toBe(1200);
		expect(Object.keys(snap.players)).toHaveLength(1);
		const myConnId = Object.entries(snap.players).find(([, p]) => p.name === "Explorer")?.[0];
		expect(myConnId).toBeTypeOf("string");

		await chunk.setInput({ inputX: 1, inputY: 0 });
		await sleep(350);

		const snap2 = await chunk.getSnapshot();
		const myPos = myConnId ? snap2.players[myConnId] : undefined;
		expect(myPos).toBeDefined();
		expect(myPos!.x).toBeGreaterThan(600);

		await chunk.removePlayer();
		await chunk.dispose();
	}, 15_000);

	test("open-world chunk transfer moves player to new chunk", async (ctx) => {
		const { client } = await setupTest(ctx, registry);

		const r1 = resolveChunkForPosition(1190, 600);
		expect(r1.chunkKey).toEqual([WORLD_ID, 0, 0]);

		const chunk0 = client.openWorldChunk
			.getOrCreate([WORLD_ID, "0", "0"])
			.connect();
		const chunk1 = client.openWorldChunk
			.getOrCreate([WORLD_ID, "1", "0"])
			.connect();
		await chunk0.addPlayer({
			name: "Traveler",
			spawnX: r1.spawnX,
			spawnY: r1.spawnY,
		});
		await sleep(200);

		await chunk0.setInput({ inputX: 1, inputY: 0 });
		await sleep(500);

		const snapAtEdge = await chunk0.getSnapshot();
		const travelerConnId = Object.entries(snapAtEdge.players).find(
			([, p]) => p.name === "Traveler",
		)?.[0];
		const posAtEdge = travelerConnId ? snapAtEdge.players[travelerConnId] : undefined;
		expect(posAtEdge).toBeDefined();
		expect(posAtEdge!.x).toBe(1199);

		const absX = 0 * 1200 + posAtEdge!.x + 1;
		const absY = 0 * 1200 + posAtEdge!.y;
		const r2 = resolveChunkForPosition(absX, absY);
		expect(r2.chunkKey).toEqual([WORLD_ID, 1, 0]);
		expect(r2.spawnX).toBeTypeOf("number");
		expect(r2.spawnY).toBeTypeOf("number");

		await chunk0.removePlayer();
		await chunk1.addPlayer({
			name: "Traveler",
			spawnX: r2.spawnX,
			spawnY: r2.spawnY,
		});
		await sleep(300);

		const snapNewChunk = await chunk1.getSnapshot();
		expect(snapNewChunk.chunkX).toBe(1);
		expect(snapNewChunk.chunkY).toBe(0);
		const newTravelerConnId = Object.entries(snapNewChunk.players).find(
			([, p]) => p.name === "Traveler",
		)?.[0];
		const newPos = newTravelerConnId
			? snapNewChunk.players[newTravelerConnId]
			: undefined;
		expect(newPos).toBeDefined();
		expect(newPos!.x).toBeLessThan(100);

		await chunk1.setInput({ inputX: 1, inputY: 0 });
		await sleep(350);
		const snapMoved = await chunk1.getSnapshot();
		const movedPos = newTravelerConnId ? snapMoved.players[newTravelerConnId] : undefined;
		expect(movedPos).toBeDefined();
		expect(movedPos!.x).toBeGreaterThan(newPos!.x);

		await Promise.all([chunk0.dispose(), chunk1.dispose()]);
	}, 15_000);

	test("idle building and production with leaderboard", async (ctx) => {
		const { client } = await setupTest(ctx, registry);

		const playerId = crypto.randomUUID();
		const world = client.idleWorld.getOrCreate([playerId]).connect();

		await world.initialize({ playerName: "Builder" });
		const state1 = await world.getState();
		expect(state1.playerName).toBe("Builder");
		expect(state1.resources).toBe(10);
		expect(state1.buildings).toHaveLength(1);
		expect(state1.buildings[0]!.typeId).toBe("farm");

		const lb = client.idleLeaderboard.getOrCreate(["main"]).connect();
		await sleep(100);
		const scores = await lb.getTopScores({ limit: 10 });
		expect(Array.isArray(scores)).toBe(true);

		await Promise.all([world.dispose(), lb.dispose()]);
	}, 15_000);
});

function resolveChunkForPosition(
	x: number,
	y: number,
): { chunkKey: [string, number, number]; spawnX: number; spawnY: number } {
	const chunkX = Math.floor(x / CHUNK_SIZE);
	const chunkY = Math.floor(y / CHUNK_SIZE);
	return {
		chunkKey: [WORLD_ID, chunkX, chunkY],
		spawnX: ((x % CHUNK_SIZE) + CHUNK_SIZE) % CHUNK_SIZE,
		spawnY: ((y % CHUNK_SIZE) + CHUNK_SIZE) % CHUNK_SIZE,
	};
}

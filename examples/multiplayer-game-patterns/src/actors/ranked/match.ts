import { type ActorContextOf, actor, event, UserError } from "rivetkit";
import { db } from "rivetkit/db";
import { interval } from "rivetkit/utils";
import type { registry } from "../index.ts";
import { getPlayerColor } from "../player-color.ts";
import {
	K_FACTOR,
	MAX_SPEED,
	SCORE_LIMIT,
	SHOOT_ANGLE,
	SHOOT_RANGE,
	SNAPSHOT_INTERVAL_MS,
	TICK_MS,
	WORLD_SIZE,
} from "./config.ts";

// How long to wait before sending a failed match report again
const REPORT_RETRY_MS = 1_000;

interface PlayerEntry {
	connId: string | null;
	color: string;
	x: number;
	y: number;
	lastPositionAt: number;
	alive: boolean;
	score: number;
	rating: number;
}

interface MatchVars {
	matchId: string;
	tick: number;
	phase: "waiting" | "live" | "finished";
	players: Record<string, PlayerEntry>;
	winnerId: string | null;
	resultReported: boolean;
	// A failed report armed a retry schedule, so the loop stops sending it.
	reportRetryScheduled: boolean;
	// `onWake` stores its loop here so `onSleep` can wait for it.
	loop: Promise<void> | null;
}

interface AssignedPlayer {
	username: string;
	rating: number;
}

type MatchInfoRow = {
	tick: number;
	phase: MatchVars["phase"];
	winner_id: string | null;
	result_reported: number;
};

type RosterRow = {
	username: string;
	rating: number;
	x: number;
	y: number;
	score: number;
};

export const rankedMatch = actor({
	options: { name: "Ranked - Match", icon: "ranking-star" },
	events: {
		snapshot: event<Snapshot>(),
		shoot: event<ShootEvent>(),
	},
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS match_info (
					id INTEGER PRIMARY KEY CHECK (id = 1),
					tick INTEGER NOT NULL DEFAULT 0,
					phase TEXT NOT NULL DEFAULT 'waiting',
					winner_id TEXT,
					result_reported INTEGER NOT NULL DEFAULT 0
				)
			`);
			await db.execute(`
				CREATE TABLE IF NOT EXISTS roster (
					username TEXT PRIMARY KEY,
					rating INTEGER NOT NULL,
					x REAL NOT NULL,
					y REAL NOT NULL,
					score INTEGER NOT NULL DEFAULT 0
				)
			`);
		},
	}),
	onCreate: async (
		c,
		input: { assignedPlayers: AssignedPlayer[] },
	) => {
		// Save the two players, their ratings, and a random spawn point once,
		// so every start of this match sees the same roster.
		await c.db.transaction(
			async (tx) => {
				await tx.execute("INSERT INTO match_info (id) VALUES (1)");
				for (const ap of input.assignedPlayers) {
					await tx.execute(
						"INSERT INTO roster (username, rating, x, y) VALUES (?, ?, ?, ?)",
						ap.username,
						ap.rating,
						Math.random() * WORLD_SIZE,
						Math.random() * WORLD_SIZE,
					);
				}
			},
			{ name: "create-match" },
		);
	},
	createVars: async (c): Promise<MatchVars> => {
		const [info] = await c.db.execute<MatchInfoRow>(
			"SELECT tick, phase, winner_id, result_reported FROM match_info WHERE id = 1",
		);
		const roster = await c.db.execute<RosterRow>(
			"SELECT username, rating, x, y, score FROM roster",
		);

		// The live duel runs in memory and starts from the last save. No
		// connection survives a restart, so both players start unbound.
		const players: Record<string, PlayerEntry> = {};
		for (const row of roster) {
			players[row.username] = {
				connId: null,
				color: getPlayerColor(row.username),
				x: row.x,
				y: row.y,
				lastPositionAt: Date.now(),
				alive: true,
				score: row.score,
				rating: row.rating,
			};
		}
		return {
			matchId: c.key[0]!,
			tick: info!.tick,
			phase: info!.phase,
			players,
			winnerId: info!.winner_id,
			resultReported: info!.result_reported === 1,
			reportRetryScheduled: false,
			loop: null,
		};
	},
	onConnect: (c, conn) => {
		const params = conn.params as { username?: string; playerId?: string };
		const playerId = params.username ?? params.playerId;
		if (!playerId) {
			conn.disconnect("invalid_player");
			return;
		}
		const player = c.vars.players[playerId];
		if (!player) {
			conn.disconnect("invalid_player");
			return;
		}
		player.connId = conn.id;

		if (c.vars.phase === "waiting") {
			const allConnected = Object.values(c.vars.players).every(
				(p) => p.connId !== null,
			);
			if (allConnected) {
				c.vars.phase = "live";
			}
		}
		broadcastSnapshot(c);
	},
	onDestroy: async (c) => {
		// `c.abortSignal` also fires on destroy. Wait for the loop to exit, so a
		// result it is still reporting is not reported a second time here.
		await c.vars.loop;
		await reportMatchCompletedIfNeeded(c, { allowNoResult: true });
	},
	onDisconnect: (c, conn) => {
		const found = findPlayerByConnId(c.vars, conn.id);
		if (!found) return;
		const [, player] = found;
		player.connId = null;
		broadcastSnapshot(c);
	},
	// docs:start tick
	// `onWake` runs on every start, so the loop restarts after a sleep or crash
	onWake: (c) => {
		c.vars.loop = runMatch(c).catch((error) => {
			c.log.error({ msg: "match loop failed", error });
		});
	},
	// `c.abortSignal` fires on shutdown, so the loop saves and exits on its own
	onSleep: async (c) => {
		await c.vars.loop;
	},
	// docs:end tick
	actions: {
		// Runs from the schedule that `reportResult` arms after a failed report.
		retryReport: async (c) => {
			if (!c.scheduledFire) {
				throw new UserError("retryReport only runs from a schedule", {
					code: "schedule_only",
				});
			}
			c.vars.reportRetryScheduled = false;
			await reportResult(c);
		},
		// docs:start validate
		updatePosition: (c, input: { x: number; y: number }) => {
			const found = findPlayerByConnId(c.vars, c.conn.id);
			if (!found) {
				return;
			}
			const [, player] = found;
			if (!player.alive) {
				throw new UserError("player is not alive", {
					code: "not_alive",
				});
			}

			const now = Date.now();
			const elapsed = Math.max(0, (now - player.lastPositionAt) / 1000);
			const maxDist = MAX_SPEED * elapsed;

			const dx = input.x - player.x;
			const dy = input.y - player.y;
			const dist = Math.sqrt(dx * dx + dy * dy);

			let newX: number;
			let newY: number;
			if (dist > maxDist && dist > 0) {
				newX = player.x + (dx / dist) * maxDist;
				newY = player.y + (dy / dist) * maxDist;
			} else {
				newX = input.x;
				newY = input.y;
			}

			player.x = Math.max(0, Math.min(WORLD_SIZE, newX));
			player.y = Math.max(0, Math.min(WORLD_SIZE, newY));
			player.lastPositionAt = now;
		},
		// docs:end validate
		shoot: (c, input: { dirX: number; dirY: number }) => {
			const found = findPlayerByConnId(c.vars, c.conn.id);
			if (!found) {
				return;
			}
			const [shooterId, shooter] = found;
			if (!shooter.alive) {
				throw new UserError("player is not alive", {
					code: "not_alive",
				});
			}
			if (c.vars.phase !== "live") {
				throw new UserError("match is not live", { code: "not_live" });
			}

			const mag = Math.sqrt(
				input.dirX * input.dirX + input.dirY * input.dirY,
			);
			if (mag === 0) return;
			const ndx = input.dirX / mag;
			const ndy = input.dirY / mag;

			let closestId: string | null = null;
			let closestDist = Infinity;

			for (const [targetId, target] of Object.entries(c.vars.players)) {
				if (targetId === shooterId) continue;
				if (!target.alive) continue;

				const tx = target.x - shooter.x;
				const ty = target.y - shooter.y;
				const targetDist = Math.sqrt(tx * tx + ty * ty);
				if (targetDist > SHOOT_RANGE || targetDist === 0) continue;

				const dot = (tx / targetDist) * ndx + (ty / targetDist) * ndy;
				const angle = Math.acos(Math.max(-1, Math.min(1, dot)));
				if (angle > SHOOT_ANGLE) continue;

				if (targetDist < closestDist) {
					closestDist = targetDist;
					closestId = targetId;
				}
			}

			if (closestId) {
				const victim = c.vars.players[closestId]!;
				shooter.score += 1;
				victim.x = Math.random() * WORLD_SIZE;
				victim.y = Math.random() * WORLD_SIZE;
				victim.lastPositionAt = Date.now();
			}

			const shootEvent: ShootEvent = {
				shooterId,
				fromX: shooter.x,
				fromY: shooter.y,
				dirX: ndx,
				dirY: ndy,
				hitPlayerId: closestId,
			};
			c.broadcast("shoot", shootEvent);

			checkWinCondition(c);
			if ((c.vars.phase as string) === "finished") {
				broadcastSnapshot(c);
			}
		},
		getSnapshot: (c) => buildSnapshot(c),
	},
});

function checkWinCondition(c: ActorContextOf<typeof rankedMatch>) {
	if (c.vars.phase !== "live") return;
	for (const [id, player] of Object.entries(c.vars.players)) {
		if (player.score >= SCORE_LIMIT) {
			c.vars.phase = "finished";
			c.vars.winnerId = id;
			return;
		}
	}
}

// docs:start elo
/** Sends the match result to the matchmaker once. Returns true if this call sent it. */
async function reportMatchCompletedIfNeeded(
	c: ActorContextOf<typeof rankedMatch>,
	options?: { allowNoResult?: boolean },
): Promise<boolean> {
	const allowNoResult = options?.allowNoResult ?? false;

	if (c.vars.resultReported) return false;
	if (c.vars.phase !== "finished" && !allowNoResult) return false;

	const client = c.client<typeof registry>();
	const entries = Object.entries(c.vars.players);
	if (c.vars.winnerId && entries.length === 2) {
		const loserUsername = entries.find(
			([id]) => id !== c.vars.winnerId,
		)?.[0];
		const winner = c.vars.players[c.vars.winnerId];
		const loser = loserUsername
			? c.vars.players[loserUsername]
			: undefined;
		if (winner && loser && loserUsername) {
			const [newWR, newLR] = calculateElo(winner.rating, loser.rating);
			await client.rankedMatchmaker.getOrCreate(["main"]).matchCompleted({
				matchId: c.vars.matchId,
				winnerUsername: c.vars.winnerId,
				loserUsername,
				winnerNewRating: newWR,
				loserNewRating: newLR,
			});
			c.vars.resultReported = true;
			return true;
		}
	}
	if (!allowNoResult) return false;

	await client.rankedMatchmaker.getOrCreate(["main"]).matchCompleted({
		matchId: c.vars.matchId,
		winnerUsername: "",
		loserUsername: "",
		winnerNewRating: 0,
		loserNewRating: 0,
	});
	c.vars.resultReported = true;
	return true;
}

function calculateElo(
	winnerRating: number,
	loserRating: number,
): [number, number] {
	const expectedW = 1 / (1 + 10 ** ((loserRating - winnerRating) / 400));
	const expectedL = 1 - expectedW;
	return [
		Math.round(winnerRating + K_FACTOR * (1 - expectedW)),
		Math.round(loserRating + K_FACTOR * (0 - expectedL)),
	];
}
// docs:end elo

function broadcastSnapshot(c: ActorContextOf<typeof rankedMatch>) {
	c.broadcast("snapshot", buildSnapshot(c));
}

// docs:start tick
/** Steps the duel every `TICK_MS` until the actor starts shutting down. */
async function runMatch(c: ActorContextOf<typeof rankedMatch>): Promise<void> {
	const tick = interval(TICK_MS);
	let savedAt = Date.now();
	while (!c.aborted) {
		await tick();
		if (c.aborted) break;

		// Ticks and actions only change memory. Save to SQLite every
		// SNAPSHOT_INTERVAL_MS in every phase, so a finished match stays
		// finished after a crash.
		const now = Date.now();
		if (now - savedAt >= SNAPSHOT_INTERVAL_MS) {
			await saveMatch(c);
			savedAt = now;
		}

		if (c.vars.phase !== "live") {
			if (!c.vars.reportRetryScheduled) await reportResult(c);
			continue;
		}
		c.vars.tick += 1;
		checkWinCondition(c);
		broadcastSnapshot(c);
	}

	// Save once more when the Actor stops, so a clean restart loses nothing.
	await saveMatch(c);
}
// docs:end tick

/**
 * Reports a finished match and saves that it did, so a restart does not send
 * the report again. The report is safe to send again, so a failure arms a
 * retry schedule instead of stopping the loop. A schedule wakes the match
 * even after both players have left and it went to sleep.
 */
async function reportResult(c: ActorContextOf<typeof rankedMatch>) {
	try {
		if (await reportMatchCompletedIfNeeded(c)) await saveMatch(c);
	} catch (error) {
		c.log.warn({ msg: "reporting the match result failed; retrying", error });
		c.vars.reportRetryScheduled = true;
		await c.schedule.after(REPORT_RETRY_MS, "retryReport");
	}
}

// docs:start save
/**
 * Writes the phase, tick, winner, whether the result was reported, and both
 * players' positions and scores to SQLite in one transaction, so a crash
 * leaves either the previous save or this one, never part of both.
 */
async function saveMatch(c: ActorContextOf<typeof rankedMatch>) {
	// Copy the values first, because actions such as updatePosition can run
	// while the transaction waits on SQLite.
	const { tick, phase, winnerId, resultReported } = c.vars;
	const players = Object.entries(c.vars.players).map(
		([username, player]) => ({
			username,
			x: player.x,
			y: player.y,
			score: player.score,
		}),
	);

	await c.db.transaction(
		async (tx) => {
			await tx.execute(
				"UPDATE match_info SET tick = ?, phase = ?, winner_id = ?, result_reported = ? WHERE id = 1",
				tick,
				phase,
				winnerId,
				resultReported ? 1 : 0,
			);
			for (const player of players) {
				await tx.execute(
					"UPDATE roster SET x = ?, y = ?, score = ? WHERE username = ?",
					player.x,
					player.y,
					player.score,
					player.username,
				);
			}
		},
		{ name: "save-match" },
	);
}
// docs:end save

interface Snapshot {
	matchId: string;
	tick: number;
	phase: "waiting" | "live" | "finished";
	winnerId: string | null;
	worldSize: number;
	scoreLimit: number;
	players: Record<
		string,
		{ x: number; y: number; color: string; score: number; rating: number }
	>;
}

interface ShootEvent {
	shooterId: string;
	fromX: number;
	fromY: number;
	dirX: number;
	dirY: number;
	hitPlayerId: string | null;
}

function buildSnapshot(c: ActorContextOf<typeof rankedMatch>): Snapshot {
	const players: Snapshot["players"] = {};
	for (const [id, entry] of Object.entries(c.vars.players)) {
		players[id] = {
			x: entry.x,
			y: entry.y,
			color: entry.color,
			score: entry.score,
			rating: entry.rating,
		};
	}
	return {
		matchId: c.vars.matchId,
		tick: c.vars.tick,
		phase: c.vars.phase,
		winnerId: c.vars.winnerId,
		worldSize: WORLD_SIZE,
		scoreLimit: SCORE_LIMIT,
		players,
	};
}

function findPlayerByConnId(
	vars: MatchVars,
	connId: string,
): [string, PlayerEntry] | null {
	for (const [id, entry] of Object.entries(vars.players)) {
		if (entry.connId === connId) return [id, entry];
	}
	return null;
}

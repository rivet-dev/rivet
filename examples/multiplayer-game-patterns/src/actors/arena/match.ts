import { type ActorContextOf, actor, event, UserError } from "rivetkit";
import { db } from "rivetkit/db";
import { interval } from "rivetkit/utils";
import type { registry } from "../index.ts";
import { getPlayerColor } from "../player-color.ts";
import {
	MAX_SPEED,
	type Mode,
	SCORE_LIMIT,
	SHOOT_ANGLE,
	SHOOT_RANGE,
	SNAPSHOT_INTERVAL_MS,
	TICK_MS,
	WORLD_SIZE,
} from "./config.ts";

interface PlayerEntry {
	connId: string | null;
	teamId: number;
	color: string;
	x: number;
	y: number;
	lastPositionAt: number;
	alive: boolean;
	score: number;
}

interface MatchVars {
	matchId: string;
	mode: Mode;
	capacity: number;
	tick: number;
	phase: "waiting" | "live" | "finished";
	players: Record<string, PlayerEntry>;
	winnerTeam: number | null;
	winnerPlayerId: string | null;
	// `onWake` stores its loop here so `onSleep` can wait for it.
	loop: Promise<void> | null;
}

interface AssignedPlayer {
	playerId: string;
	teamId: number;
}

type MatchInfoRow = {
	match_id: string;
	mode: Mode;
	capacity: number;
	tick: number;
	phase: MatchVars["phase"];
	winner_team: number | null;
	winner_player_id: string | null;
};

type RosterRow = {
	player_id: string;
	team_id: number;
	x: number;
	y: number;
	score: number;
};

export const arenaMatch = actor({
	options: { name: "Arena - Match", icon: "crosshairs" },
	events: {
		snapshot: event<Snapshot>(),
		shoot: event<ShootEvent>(),
	},
	// docs:start connect
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS match_info (
					id INTEGER PRIMARY KEY CHECK (id = 1),
					match_id TEXT NOT NULL,
					mode TEXT NOT NULL,
					capacity INTEGER NOT NULL,
					tick INTEGER NOT NULL DEFAULT 0,
					phase TEXT NOT NULL DEFAULT 'waiting',
					winner_team INTEGER,
					winner_player_id TEXT
				)
			`);
			await db.execute(`
				CREATE TABLE IF NOT EXISTS roster (
					player_id TEXT PRIMARY KEY,
					team_id INTEGER NOT NULL,
					x REAL NOT NULL,
					y REAL NOT NULL,
					score INTEGER NOT NULL DEFAULT 0
				)
			`);
		},
	}),
	onCreate: async (
		c,
		input: {
			matchId: string;
			mode: Mode;
			capacity: number;
			assignedPlayers: AssignedPlayer[];
		},
	) => {
		// Save the settings and the roster once, with a random spawn point for
		// each player, so every start of this match sees the same players.
		await c.db.transaction(
			async (tx) => {
				await tx.execute(
					"INSERT INTO match_info (id, match_id, mode, capacity) VALUES (1, ?, ?, ?)",
					input.matchId,
					input.mode,
					input.capacity,
				);
				for (const ap of input.assignedPlayers) {
					await tx.execute(
						"INSERT INTO roster (player_id, team_id, x, y) VALUES (?, ?, ?, ?)",
						ap.playerId,
						ap.teamId,
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
			"SELECT match_id, mode, capacity, tick, phase, winner_team, winner_player_id FROM match_info WHERE id = 1",
		);
		const roster = await c.db.execute<RosterRow>(
			"SELECT player_id, team_id, x, y, score FROM roster",
		);

		// The live match runs in memory and starts from the last save. No
		// connection survives a restart, so every player starts unbound.
		const players: Record<string, PlayerEntry> = {};
		for (const row of roster) {
			players[row.player_id] = {
				connId: null,
				teamId: row.team_id,
				color: getPlayerColor(row.player_id),
				x: row.x,
				y: row.y,
				lastPositionAt: Date.now(),
				alive: true,
				score: row.score,
			};
		}
		return {
			matchId: info!.match_id,
			mode: info!.mode,
			capacity: info!.capacity,
			tick: info!.tick,
			phase: info!.phase,
			players,
			winnerTeam: info!.winner_team,
			winnerPlayerId: info!.winner_player_id,
			loop: null,
		};
	},
	onConnect: (c, conn) => {
		const playerId = (conn.params as { playerId?: string })?.playerId;
		if (!playerId) return;
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
	// docs:end connect
	onDestroy: async (c) => {
		// `c.abortSignal` also fires on destroy. Let the loop finish its last
		// save before the database closes.
		await c.vars.loop;
		const client = c.client<typeof registry>();
		await client.arenaMatchmaker
			.getOrCreate(["main"])
			.matchCompleted({ matchId: c.vars.matchId });
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
		// docs:start combat
		updatePosition: (c, input: { x: number; y: number }) => {
			const found = findPlayerByConnId(c.vars, c.conn.id);
			if (!found) {
				throw new UserError("player not found", {
					code: "player_not_found",
				});
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
		shoot: (c, input: { dirX: number; dirY: number }) => {
			const found = findPlayerByConnId(c.vars, c.conn.id);
			if (!found) {
				throw new UserError("player not found", {
					code: "player_not_found",
				});
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
				if (shooter.teamId >= 0 && target.teamId === shooter.teamId)
					continue;

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
		// docs:end combat
		getSnapshot: (c) => buildSnapshot(c),
	},
});

// docs:start combat
function checkWinCondition(c: ActorContextOf<typeof arenaMatch>) {
	if (c.vars.phase !== "live") return;

	if (c.vars.mode === "ffa") {
		for (const [id, player] of Object.entries(c.vars.players)) {
			if (player.score >= SCORE_LIMIT) {
				c.vars.phase = "finished";
				c.vars.winnerPlayerId = id;
				return;
			}
		}
	} else {
		const teamScores: Record<number, number> = {};
		for (const player of Object.values(c.vars.players)) {
			teamScores[player.teamId] =
				(teamScores[player.teamId] ?? 0) + player.score;
		}
		for (const [teamId, score] of Object.entries(teamScores)) {
			if (score >= SCORE_LIMIT) {
				c.vars.phase = "finished";
				c.vars.winnerTeam = Number(teamId);
				return;
			}
		}
	}
}
// docs:end combat

function broadcastSnapshot(c: ActorContextOf<typeof arenaMatch>) {
	c.broadcast("snapshot", buildSnapshot(c));
}

// docs:start tick
/** Steps the match every `TICK_MS` until the actor starts shutting down. */
async function runMatch(c: ActorContextOf<typeof arenaMatch>): Promise<void> {
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

		if (c.vars.phase !== "live") continue;

		c.vars.tick += 1;

		checkWinCondition(c);

		broadcastSnapshot(c);
	}

	// Save once more when the Actor stops, so a clean restart loses nothing.
	await saveMatch(c);
}
// docs:end tick

// docs:start save
/**
 * Writes the phase, tick, winner, and every player's position and score to
 * SQLite in one transaction, so a crash leaves either the previous save or
 * this one, never part of both.
 */
async function saveMatch(c: ActorContextOf<typeof arenaMatch>) {
	// Copy the values first, because actions such as updatePosition can run
	// while the transaction waits on SQLite.
	const { tick, phase, winnerTeam, winnerPlayerId } = c.vars;
	const players = Object.entries(c.vars.players).map(
		([playerId, player]) => ({
			playerId,
			x: player.x,
			y: player.y,
			score: player.score,
		}),
	);

	await c.db.transaction(
		async (tx) => {
			await tx.execute(
				"UPDATE match_info SET tick = ?, phase = ?, winner_team = ?, winner_player_id = ? WHERE id = 1",
				tick,
				phase,
				winnerTeam,
				winnerPlayerId,
			);
			for (const player of players) {
				await tx.execute(
					"UPDATE roster SET x = ?, y = ?, score = ? WHERE player_id = ?",
					player.x,
					player.y,
					player.score,
					player.playerId,
				);
			}
		},
		{ name: "save-match" },
	);
}
// docs:end save

interface Snapshot {
	matchId: string;
	mode: Mode;
	capacity: number;
	tick: number;
	phase: "waiting" | "live" | "finished";
	winnerTeam: number | null;
	winnerPlayerId: string | null;
	worldSize: number;
	scoreLimit: number;
	players: Record<
		string,
		{ x: number; y: number; teamId: number; color: string; score: number }
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

function buildSnapshot(c: ActorContextOf<typeof arenaMatch>): Snapshot {
	const players: Record<
		string,
		{ x: number; y: number; teamId: number; color: string; score: number }
	> = {};
	for (const [id, entry] of Object.entries(c.vars.players)) {
		players[id] = {
			x: entry.x,
			y: entry.y,
			teamId: entry.teamId,
			color: entry.color,
			score: entry.score,
		};
	}
	return {
		matchId: c.vars.matchId,
		mode: c.vars.mode,
		capacity: c.vars.capacity,
		tick: c.vars.tick,
		phase: c.vars.phase,
		winnerTeam: c.vars.winnerTeam,
		winnerPlayerId: c.vars.winnerPlayerId,
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

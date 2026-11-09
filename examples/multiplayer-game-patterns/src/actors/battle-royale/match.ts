import { type ActorContextOf, actor, event, UserError } from "rivetkit";
import { db } from "rivetkit/db";
import { interval } from "rivetkit/utils";
import type { registry } from "../index.ts";
import { getPlayerColor } from "../player-color.ts";
import {
	LOBBY_CAPACITY,
	LOBBY_COUNTDOWN_TICKS,
	MAX_SPEED,
	PLAYER_MAX_HP,
	SHOOT_ANGLE,
	SHOOT_RANGE,
	SNAPSHOT_INTERVAL_MS,
	TICK_MS,
	WORLD_SIZE,
	ZONE_DAMAGE_PER_TICK,
	ZONE_INITIAL_RADIUS,
	ZONE_MIN_RADIUS,
	ZONE_SHRINK_RATE,
	ZONE_SHRINK_START_TICK,
} from "./config.ts";

interface PlayerEntry {
	connId: string | null;
	color: string;
	x: number;
	y: number;
	lastPositionAt: number;
	hp: number;
	alive: boolean;
	placement: number | null;
	disconnectedAt: number | null;
}

interface MatchVars {
	matchId: string;
	tick: number;
	phase: "lobby" | "live" | "finished";
	players: Record<string, PlayerEntry>;
	zone: { centerX: number; centerY: number; radius: number };
	eliminationOrder: string[];
	winnerId: string | null;
	lobbyCountdown: number | null;
	// `onWake` stores its loop here so `onSleep` can wait for it.
	loop: Promise<void> | null;
}

interface ConnParams {
	playerId?: string;
}

type MatchInfoRow = {
	tick: number;
	phase: MatchVars["phase"];
	lobby_countdown: number | null;
	zone_center_x: number;
	zone_center_y: number;
	zone_radius: number;
	winner_id: string | null;
};

type PlayerRow = {
	player_id: string;
	x: number;
	y: number;
	hp: number;
	alive: number;
	placement: number | null;
};

export const battleRoyaleMatch = actor({
	options: { name: "Battle Royale - Match", icon: "skull-crossbones" },
	events: {
		snapshot: event<Snapshot>(),
		shoot: event<ShootEvent>(),
	},
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS match_info (
					id INTEGER PRIMARY KEY CHECK (id = 1),
					tick INTEGER NOT NULL,
					phase TEXT NOT NULL,
					lobby_countdown INTEGER,
					zone_center_x REAL NOT NULL,
					zone_center_y REAL NOT NULL,
					zone_radius REAL NOT NULL,
					winner_id TEXT
				)
			`);
			await db.execute(`
				CREATE TABLE IF NOT EXISTS players (
					player_id TEXT PRIMARY KEY,
					x REAL NOT NULL,
					y REAL NOT NULL,
					hp INTEGER NOT NULL,
					alive INTEGER NOT NULL,
					placement INTEGER,
					elimination_index INTEGER
				)
			`);
		},
	}),
	// The match runs in memory and starts from the last save. Its key is the
	// match id.
	createVars: async (c): Promise<MatchVars> => {
		const matchId = c.key[0]!;
		const [info] = await c.db.execute<MatchInfoRow>(
			"SELECT tick, phase, lobby_countdown, zone_center_x, zone_center_y, zone_radius, winner_id FROM match_info WHERE id = 1",
		);
		if (!info) {
			return {
				matchId,
				tick: 0,
				phase: "lobby",
				players: {},
				zone: {
					centerX: WORLD_SIZE / 2,
					centerY: WORLD_SIZE / 2,
					radius: ZONE_INITIAL_RADIUS,
				},
				eliminationOrder: [],
				winnerId: null,
				lobbyCountdown: null,
				loop: null,
			};
		}

		const rows = await c.db.execute<PlayerRow>(
			"SELECT player_id, x, y, hp, alive, placement FROM players",
		);
		const eliminated = await c.db.execute<{ player_id: string }>(
			"SELECT player_id FROM players WHERE elimination_index IS NOT NULL ORDER BY elimination_index",
		);

		// No connection survives a restart, so every claimed player starts
		// disconnected. The lobby drops the ones who don't reconnect in time.
		const players: Record<string, PlayerEntry> = {};
		for (const row of rows) {
			players[row.player_id] = {
				connId: null,
				color: getPlayerColor(row.player_id),
				x: row.x,
				y: row.y,
				lastPositionAt: Date.now(),
				hp: row.hp,
				alive: row.alive === 1,
				placement: row.placement,
				disconnectedAt: Date.now(),
			};
		}
		return {
			matchId,
			tick: info.tick,
			phase: info.phase,
			players,
			zone: {
				centerX: info.zone_center_x,
				centerY: info.zone_center_y,
				radius: info.zone_radius,
			},
			eliminationOrder: eliminated.map((row) => row.player_id),
			winnerId: info.winner_id,
			lobbyCountdown: info.lobby_countdown,
			loop: null,
		};
	},
	// docs:start connect
	onConnect: async (c, conn) => {
		const params = (conn.params as ConnParams | null) ?? {};
		const playerId = params.playerId;
		if (!playerId) {
			conn.disconnect("invalid_player");
			return;
		}
		const existingPlayer = c.vars.players[playerId];
		if (existingPlayer) {
			existingPlayer.connId = conn.id;
			existingPlayer.disconnectedAt = null;
		} else {
			const confirmed = await claimPendingPlayer(c, playerId);
			if (!confirmed) {
				conn.disconnect("player_not_pending");
				return;
			}
			const pos = randomPositionInZone(c.vars.zone);
			c.vars.players[playerId] = {
				connId: conn.id,
				color: getPlayerColor(playerId),
				x: pos.x,
				y: pos.y,
				lastPositionAt: Date.now(),
				hp: PLAYER_MAX_HP,
				alive: true,
				placement: null,
				disconnectedAt: null,
			};

			// The matchmaker has used up this player's reservation, so save
			// the player now. A crash before the next save would otherwise
			// turn them away with player_not_pending when they reconnect.
			await saveMatch(c);
		}

		await updateMatchmaker(c);
		broadcastSnapshot(c);
	},
	// docs:end connect
	onDisconnect: async (c, conn) => {
		const found = findPlayerByConnId(c.vars, conn.id);
		if (!found) return;
		const [, player] = found;
		player.connId = null;
		player.disconnectedAt = Date.now();

		await updateMatchmaker(c);
		broadcastSnapshot(c);
	},
	onDestroy: async (c) => {
		// `c.abortSignal` also fires on destroy. Let the loop finish its last
		// save before the database closes.
		await c.vars.loop;
		const client = c.client<typeof registry>();
		await client.battleRoyaleMatchmaker
			.getOrCreate(["main"])
			.closeMatch({ matchId: c.vars.matchId });
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
			if (!player.alive) return;

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
			if (c.vars.phase !== "live") {
				throw new UserError("combat not active", { code: "not_live" });
			}
			const found = findPlayerByConnId(c.vars, c.conn.id);
			if (!found) {
				throw new UserError("player not found", {
					code: "player_not_found",
				});
			}
			const [shooterId, shooter] = found;
			if (!shooter.alive) return;

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
				victim.hp -= 3;
				if (victim.hp <= 0) {
					victim.hp = 0;
					victim.alive = false;
					c.vars.eliminationOrder.push(closestId);
					victim.placement = countAlivePlayers(c.vars) + 1;
				}
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

			const alivePlayers = Object.entries(c.vars.players).filter(
				([, p]) => p.alive,
			);
			if (alivePlayers.length <= 1) {
				c.vars.phase = "finished";
				if (alivePlayers.length === 1) {
					const [winnerId, winner] = alivePlayers[0]!;
					c.vars.winnerId = winnerId;
					winner.placement = 1;
				}
				broadcastSnapshot(c);
			}
		},
		// docs:end combat
		getSnapshot: (c) => buildSnapshot(c),
	},
});

// docs:start tick
/** Steps the match every `TICK_MS` until the actor starts shutting down. */
async function runMatch(
	c: ActorContextOf<typeof battleRoyaleMatch>,
): Promise<void> {
	const tick = interval(TICK_MS);
	let savedAt = Date.now();
	while (!c.aborted) {
		await tick();
		if (c.aborted) break;

		c.vars.tick += 1;
		const now = Date.now();

		// The 5 second grace is much shorter than the idle timeout, so an
		// empty lobby removes its last players here before it goes to sleep.
		if (c.vars.phase === "lobby") {
			let removedPlayer = false;
			for (const [id, player] of Object.entries(c.vars.players)) {
				if (
					player.disconnectedAt &&
					now - player.disconnectedAt > 5000
				) {
					delete c.vars.players[id];
					removedPlayer = true;
				}
			}
			if (removedPlayer) {
				await updateMatchmaker(c);
			}
		}

		if (c.vars.phase === "lobby") {
			const connectedCount = Object.values(c.vars.players).filter(
				(p) => p.connId !== null,
			).length;
			if (
				connectedCount >= LOBBY_CAPACITY &&
				c.vars.lobbyCountdown === null
			) {
				c.vars.lobbyCountdown = LOBBY_COUNTDOWN_TICKS;
			}
			if (c.vars.lobbyCountdown !== null) {
				c.vars.lobbyCountdown -= 1;
				if (c.vars.lobbyCountdown <= 0) {
					startGame(c);
				}
			}
		}

		if (c.vars.phase === "live") {
			if (c.vars.tick > ZONE_SHRINK_START_TICK) {
				c.vars.zone.radius = Math.max(
					ZONE_MIN_RADIUS,
					ZONE_INITIAL_RADIUS -
						ZONE_SHRINK_RATE *
							(c.vars.tick - ZONE_SHRINK_START_TICK),
				);
			}

			for (const [id, player] of Object.entries(c.vars.players)) {
				if (!player.alive) continue;
				const dx = player.x - c.vars.zone.centerX;
				const dy = player.y - c.vars.zone.centerY;
				const dist = Math.sqrt(dx * dx + dy * dy);
				if (dist > c.vars.zone.radius) {
					player.hp -= ZONE_DAMAGE_PER_TICK;
					if (player.hp <= 0) {
						player.hp = 0;
						player.alive = false;
						c.vars.eliminationOrder.push(id);
						player.placement = countAlivePlayers(c.vars) + 1;
					}
				}
			}

			const alivePlayers = Object.entries(c.vars.players).filter(
				([, p]) => p.alive,
			);
			if (alivePlayers.length <= 1) {
				c.vars.phase = "finished";
				if (alivePlayers.length === 1) {
					const [winnerId, winner] = alivePlayers[0]!;
					c.vars.winnerId = winnerId;
					winner.placement = 1;
				}
			}
		}

		broadcastSnapshot(c);

		// Ticks and actions only change memory. Save to SQLite every
		// SNAPSHOT_INTERVAL_MS instead of on every tick.
		if (now - savedAt >= SNAPSHOT_INTERVAL_MS) {
			await saveMatch(c);
			savedAt = now;
		}
	}

	// Save once more when the Actor stops, so a clean restart loses nothing.
	await saveMatch(c);
}
// docs:end tick

function startGame(c: ActorContextOf<typeof battleRoyaleMatch>) {
	c.vars.phase = "live";
	c.vars.tick = 0;
	c.vars.lobbyCountdown = null;

	c.vars.zone = {
		centerX: WORLD_SIZE / 2,
		centerY: WORLD_SIZE / 2,
		radius: ZONE_INITIAL_RADIUS,
	};

	for (const player of Object.values(c.vars.players)) {
		const pos = randomPositionInZone(c.vars.zone);
		player.x = pos.x;
		player.y = pos.y;
		player.hp = PLAYER_MAX_HP;
		player.alive = true;
		player.lastPositionAt = Date.now();
	}
}

function randomPositionInZone(zone: {
	centerX: number;
	centerY: number;
	radius: number;
}): { x: number; y: number } {
	const angle = Math.random() * Math.PI * 2;
	const r = Math.sqrt(Math.random()) * zone.radius * 0.9;
	return {
		x: Math.max(
			0,
			Math.min(WORLD_SIZE, zone.centerX + Math.cos(angle) * r),
		),
		y: Math.max(
			0,
			Math.min(WORLD_SIZE, zone.centerY + Math.sin(angle) * r),
		),
	};
}

/**
 * Tells the matchmaker how full this match is. The count only steers new
 * players to a lobby, and the next join or leave sends it again, so a failed
 * update is logged instead of stopping the caller or the tick loop.
 */
async function updateMatchmaker(c: ActorContextOf<typeof battleRoyaleMatch>) {
	const client = c.client<typeof registry>();
	try {
		await client.battleRoyaleMatchmaker.getOrCreate(["main"]).updateMatch({
			matchId: c.vars.matchId,
			connectedPlayerCount: Object.keys(c.vars.players).length,
			isStarted: c.vars.phase !== "lobby",
		});
	} catch (error) {
		c.log.warn({ msg: "updating the matchmaker failed", error });
	}
}

// docs:start connect
async function claimPendingPlayer(
	c: ActorContextOf<typeof battleRoyaleMatch>,
	playerId: string,
): Promise<boolean> {
	const client = c.client<typeof registry>();
	const result = await client.battleRoyaleMatchmaker
		.getOrCreate(["main"])
		.pendingPlayerConnected({ matchId: c.vars.matchId, playerId });
	return result.accepted;
}
// docs:end connect

// docs:start save
/**
 * Writes the match and every claimed player to SQLite in one transaction, so
 * a crash leaves either the previous save or this one, never part of both.
 */
async function saveMatch(c: ActorContextOf<typeof battleRoyaleMatch>) {
	// Copy the values first, because actions and ticks can run while the
	// transaction waits on SQLite.
	const { tick, phase, lobbyCountdown, winnerId } = c.vars;
	const zone = { ...c.vars.zone };
	const players = Object.entries(c.vars.players).map(([playerId, player]) => {
		const eliminationIndex = c.vars.eliminationOrder.indexOf(playerId);
		return {
			playerId,
			x: player.x,
			y: player.y,
			hp: player.hp,
			alive: player.alive ? 1 : 0,
			placement: player.placement,
			eliminationIndex: eliminationIndex === -1 ? null : eliminationIndex,
		};
	});

	await c.db.transaction(
		async (tx) => {
			await tx.execute(
				"INSERT OR REPLACE INTO match_info (id, tick, phase, lobby_countdown, zone_center_x, zone_center_y, zone_radius, winner_id) VALUES (1, ?, ?, ?, ?, ?, ?, ?)",
				tick,
				phase,
				lobbyCountdown,
				zone.centerX,
				zone.centerY,
				zone.radius,
				winnerId,
			);
			await tx.execute("DELETE FROM players");
			for (const player of players) {
				await tx.execute(
					"INSERT INTO players (player_id, x, y, hp, alive, placement, elimination_index) VALUES (?, ?, ?, ?, ?, ?, ?)",
					player.playerId,
					player.x,
					player.y,
					player.hp,
					player.alive,
					player.placement,
					player.eliminationIndex,
				);
			}
		},
		{ name: "save-match" },
	);
}
// docs:end save

function countAlivePlayers(vars: MatchVars): number {
	return Object.values(vars.players).filter((p) => p.alive).length;
}

interface Snapshot {
	matchId: string;
	tick: number;
	phase: "lobby" | "live" | "finished";
	capacity: number;
	playerCount: number;
	aliveCount: number;
	winnerId: string | null;
	lobbyCountdown: number | null;
	worldSize: number;
	zone: { centerX: number; centerY: number; radius: number };
	players: Record<
		string,
		{
			x: number;
			y: number;
			color: string;
			hp: number;
			maxHp: number;
			alive: boolean;
			placement: number | null;
		}
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

function broadcastSnapshot(c: ActorContextOf<typeof battleRoyaleMatch>) {
	c.broadcast("snapshot", buildSnapshot(c));
}

function buildSnapshot(c: ActorContextOf<typeof battleRoyaleMatch>): Snapshot {
	const players: Snapshot["players"] = {};
	for (const [id, entry] of Object.entries(c.vars.players)) {
		players[id] = {
			x: entry.x,
			y: entry.y,
			color: entry.color,
			hp: entry.hp,
			maxHp: PLAYER_MAX_HP,
			alive: entry.alive,
			placement: entry.placement,
		};
	}
	return {
		matchId: c.vars.matchId,
		tick: c.vars.tick,
		phase: c.vars.phase,
		capacity: LOBBY_CAPACITY,
		playerCount: Object.values(c.vars.players).filter(
			(p) => p.connId !== null,
		).length,
		aliveCount: countAlivePlayers(c.vars),
		winnerId: c.vars.winnerId,
		lobbyCountdown: c.vars.lobbyCountdown,
		worldSize: WORLD_SIZE,
		zone: { ...c.vars.zone },
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

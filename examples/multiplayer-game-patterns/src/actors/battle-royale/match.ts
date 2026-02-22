import { actor, type ActorContextOf, event, UserError } from "rivetkit";
import { interval } from "rivetkit/utils";
import {
	hasInvalidInternalToken,
	INTERNAL_TOKEN,
	isInternalToken,
} from "../../auth.ts";
import { registry } from "../index.ts";
import {
	LOBBY_CAPACITY,
	TICK_MS,
	WORLD_SIZE,
	MAX_SPEED,
	SHOOT_RANGE,
	SHOOT_ANGLE,
	ZONE_INITIAL_RADIUS,
	ZONE_SHRINK_START_TICK,
	ZONE_SHRINK_RATE,
	ZONE_MIN_RADIUS,
	ZONE_DAMAGE_PER_TICK,
	PLAYER_MAX_HP,
	LOBBY_COUNTDOWN_TICKS,
} from "./config.ts";

interface PlayerEntry {
	token: string;
	connId: string | null;
	x: number;
	y: number;
	lastPositionAt: number;
	hp: number;
	alive: boolean;
	placement: number | null;
	disconnectedAt: number | null;
}

interface State {
	matchId: string;
	tick: number;
	phase: "lobby" | "live" | "finished";
	players: Record<string, PlayerEntry>;
	zone: { centerX: number; centerY: number; radius: number };
	eliminationOrder: string[];
	winnerId: string | null;
	lobbyCountdown: number | null;
}

export const battleRoyaleMatch = actor({
	options: { name: "Battle Royale - Match", icon: "skull-crossbones" },
	events: {
		snapshot: event<Snapshot>(),
		shoot: event<ShootEvent>(),
	},
	createState: (_c, input: { matchId: string }): State => ({
		matchId: input.matchId,
		tick: 0,
		phase: "lobby",
		players: {},
		zone: { centerX: WORLD_SIZE / 2, centerY: WORLD_SIZE / 2, radius: ZONE_INITIAL_RADIUS },
		eliminationOrder: [],
		winnerId: null,
		lobbyCountdown: null,
	}),
	onBeforeConnect: (
		c,
		params: { playerToken?: string; internalToken?: string },
	) => {
		if (hasInvalidInternalToken(params)) {
			throw new UserError("forbidden", { code: "forbidden" });
		}
		if (params?.internalToken === INTERNAL_TOKEN) return;
		const playerToken = params?.playerToken?.trim();
		if (!playerToken) {
			throw new UserError("authentication required", { code: "auth_required" });
		}
		if (!findPlayerByToken(c.state, playerToken)) {
			throw new UserError("invalid player token", { code: "invalid_player_token" });
		}
	},
	canInvoke: (c, invoke) => {
		const isInternal = isInternalToken(
			c.conn.params as { internalToken?: string } | undefined,
		);
		const isAssignedPlayer = findPlayerByConnId(c.state, c.conn.id) !== null;
		if (invoke.kind === "action" && invoke.name === "createPlayer") {
			return isInternal;
		}
		if (
			invoke.kind === "action" &&
			(invoke.name === "updatePosition" ||
				invoke.name === "shoot" ||
				invoke.name === "getSnapshot")
		) {
			return !isInternal && isAssignedPlayer;
		}
		if (
			invoke.kind === "subscribe" &&
			(invoke.name === "snapshot" || invoke.name === "shoot")
		) {
			return !isInternal && isAssignedPlayer;
		}
		return false;
	},
	onConnect: async (c, conn) => {
		const playerToken = conn.params?.playerToken?.trim();
		if (!playerToken) return;
		const found = findPlayerByToken(c.state, playerToken);
		if (!found) {
			conn.disconnect("invalid_player_token");
			return;
		}
		const [, player] = found;
		player.connId = conn.id;
		player.disconnectedAt = null;

		await updateMatchmaker(c);
		broadcastSnapshot(c);
	},
	onDisconnect: async (c, conn) => {
		const found = findPlayerByConnId(c.state, conn.id);
		if (!found) return;
		const [, player] = found;
		player.connId = null;
		player.disconnectedAt = Date.now();

		await updateMatchmaker(c);
		broadcastSnapshot(c);
	},
	onDestroy: async (c) => {
		const client = c.client<typeof registry>();
		await client.battleRoyaleMatchmaker
			.getOrCreate(["main"], { params: { internalToken: INTERNAL_TOKEN } })
			.send("closeMatch", { matchId: c.state.matchId });
	},
	run: async (c) => {
		const tick = interval(TICK_MS);
		while (!c.aborted) {
			await tick();
			if (c.aborted) break;

			c.state.tick += 1;
			const now = Date.now();

			// Remove players disconnected > 5s in lobby phase.
			if (c.state.phase === "lobby") {
				for (const [id, player] of Object.entries(c.state.players)) {
					if (
						player.disconnectedAt &&
						now - player.disconnectedAt > 5000
					) {
						delete c.state.players[id];
					}
				}
			}

			// Lobby countdown logic.
			if (c.state.phase === "lobby") {
				const connectedCount = Object.values(c.state.players).filter(
					(p) => p.connId !== null,
				).length;
				if (connectedCount >= LOBBY_CAPACITY && c.state.lobbyCountdown === null) {
					c.state.lobbyCountdown = LOBBY_COUNTDOWN_TICKS;
				}
				if (c.state.lobbyCountdown !== null) {
					c.state.lobbyCountdown -= 1;
					if (c.state.lobbyCountdown <= 0) {
						startGame(c);
					}
				}
			}

			// Live phase: zone shrink + damage.
			if (c.state.phase === "live") {
				if (c.state.tick > ZONE_SHRINK_START_TICK) {
					c.state.zone.radius = Math.max(
						ZONE_MIN_RADIUS,
						ZONE_INITIAL_RADIUS - ZONE_SHRINK_RATE * (c.state.tick - ZONE_SHRINK_START_TICK),
					);
				}

				// Zone damage.
				for (const [id, player] of Object.entries(c.state.players)) {
					if (!player.alive) continue;
					const dx = player.x - c.state.zone.centerX;
					const dy = player.y - c.state.zone.centerY;
					const dist = Math.sqrt(dx * dx + dy * dy);
					if (dist > c.state.zone.radius) {
						player.hp -= ZONE_DAMAGE_PER_TICK;
						if (player.hp <= 0) {
							player.hp = 0;
							player.alive = false;
							c.state.eliminationOrder.push(id);
							player.placement = countAlivePlayers(c.state) + 1;
						}
					}
				}

				// Check for winner.
				const alivePlayers = Object.entries(c.state.players).filter(
					([, p]) => p.alive,
				);
				if (alivePlayers.length <= 1) {
					c.state.phase = "finished";
					if (alivePlayers.length === 1) {
						const [winnerId, winner] = alivePlayers[0]!;
						c.state.winnerId = winnerId;
						winner.placement = 1;
					}
				}
			}

			broadcastSnapshot(c);
		}
	},
	actions: {
		createPlayer: (c, input: { playerId: string; playerToken: string }) => {
			const pos = randomPositionInZone(c.state.zone);
			c.state.players[input.playerId] = {
				token: input.playerToken,
				connId: null,
				x: pos.x,
				y: pos.y,
				lastPositionAt: Date.now(),
				hp: PLAYER_MAX_HP,
				alive: true,
				placement: null,
				disconnectedAt: Date.now(),
			};
		},
		updatePosition: (c, input: { x: number; y: number }) => {
			const found = findPlayerByConnId(c.state, c.conn.id);
			if (!found) {
				throw new UserError("player not found", { code: "player_not_found" });
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
			if (c.state.phase !== "live") {
				throw new UserError("combat not active", { code: "not_live" });
			}
			const found = findPlayerByConnId(c.state, c.conn.id);
			if (!found) {
				throw new UserError("player not found", { code: "player_not_found" });
			}
			const [shooterId, shooter] = found;
			if (!shooter.alive) return;

			const mag = Math.sqrt(input.dirX * input.dirX + input.dirY * input.dirY);
			if (mag === 0) return;
			const ndx = input.dirX / mag;
			const ndy = input.dirY / mag;

			let closestId: string | null = null;
			let closestDist = Infinity;

			for (const [targetId, target] of Object.entries(c.state.players)) {
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
				const victim = c.state.players[closestId]!;
				victim.hp -= 3;
				if (victim.hp <= 0) {
					victim.hp = 0;
					victim.alive = false;
					c.state.eliminationOrder.push(closestId);
					victim.placement = countAlivePlayers(c.state) + 1;
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

			// Check for winner after elimination.
			const alivePlayers = Object.entries(c.state.players).filter(
				([, p]) => p.alive,
			);
			if (alivePlayers.length <= 1) {
				c.state.phase = "finished";
				if (alivePlayers.length === 1) {
					const [winnerId, winner] = alivePlayers[0]!;
					c.state.winnerId = winnerId;
					winner.placement = 1;
				}
				broadcastSnapshot(c);
			}
		},
		getSnapshot: (c) => buildSnapshot(c),
	},
});

function startGame(c: ActorContextOf<typeof battleRoyaleMatch>) {
	c.state.phase = "live";
	c.state.tick = 0;
	c.state.lobbyCountdown = null;

	c.state.zone = {
		centerX: WORLD_SIZE / 2,
		centerY: WORLD_SIZE / 2,
		radius: ZONE_INITIAL_RADIUS,
	};

	// Reset positions to random spots within the zone.
	for (const player of Object.values(c.state.players)) {
		const pos = randomPositionInZone(c.state.zone);
		player.x = pos.x;
		player.y = pos.y;
		player.hp = PLAYER_MAX_HP;
		player.alive = true;
		player.lastPositionAt = Date.now();
	}
}

function randomPositionInZone(zone: { centerX: number; centerY: number; radius: number }): { x: number; y: number } {
	const angle = Math.random() * Math.PI * 2;
	const r = Math.sqrt(Math.random()) * zone.radius * 0.9;
	return {
		x: Math.max(0, Math.min(WORLD_SIZE, zone.centerX + Math.cos(angle) * r)),
		y: Math.max(0, Math.min(WORLD_SIZE, zone.centerY + Math.sin(angle) * r)),
	};
}

async function updateMatchmaker(c: ActorContextOf<typeof battleRoyaleMatch>) {
	const client = c.client<typeof registry>();
	await client.battleRoyaleMatchmaker
		.getOrCreate(["main"], { params: { internalToken: INTERNAL_TOKEN } })
		.send("updateMatch", {
			matchId: c.state.matchId,
			playerCount: Object.values(c.state.players).filter((p) => p.connId !== null).length,
			isStarted: c.state.phase !== "lobby",
		});
}

function countAlivePlayers(state: State): number {
	return Object.values(state.players).filter((p) => p.alive).length;
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
	players: Record<string, { x: number; y: number; hp: number; maxHp: number; alive: boolean; placement: number | null }>;
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
	for (const [id, entry] of Object.entries(c.state.players)) {
		players[id] = {
			x: entry.x,
			y: entry.y,
			hp: entry.hp,
			maxHp: PLAYER_MAX_HP,
			alive: entry.alive,
			placement: entry.placement,
		};
	}
	return {
		matchId: c.state.matchId,
		tick: c.state.tick,
		phase: c.state.phase,
		capacity: LOBBY_CAPACITY,
		playerCount: Object.values(c.state.players).filter((p) => p.connId !== null).length,
		aliveCount: countAlivePlayers(c.state),
		winnerId: c.state.winnerId,
		lobbyCountdown: c.state.lobbyCountdown,
		worldSize: WORLD_SIZE,
		zone: { ...c.state.zone },
		players,
	};
}

function findPlayerByToken(
	state: State,
	token: string,
): [string, PlayerEntry] | null {
	for (const [id, entry] of Object.entries(state.players)) {
		if (entry.token === token) return [id, entry];
	}
	return null;
}

function findPlayerByConnId(
	state: State,
	connId: string,
): [string, PlayerEntry] | null {
	for (const [id, entry] of Object.entries(state.players)) {
		if (entry.connId === connId) return [id, entry];
	}
	return null;
}

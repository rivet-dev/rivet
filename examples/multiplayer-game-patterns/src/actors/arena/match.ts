import { actor, type ActorContextOf, event, UserError } from "rivetkit";
import { interval } from "rivetkit/utils";
import { INTERNAL_TOKEN } from "../../auth.ts";
import { registry } from "../index.ts";
import {
	type Mode,
	TICK_MS,
	WORLD_SIZE,
	MAX_SPEED,
	SHOOT_RANGE,
	SHOOT_ANGLE,
	SCORE_LIMIT,
} from "./config.ts";

interface PlayerEntry {
	token: string;
	connId: string | null;
	teamId: number; // -1 for FFA
	x: number;
	y: number;
	lastPositionAt: number;
	alive: boolean;
	score: number;
}

interface State {
	matchId: string;
	mode: Mode;
	capacity: number;
	tick: number;
	phase: "waiting" | "live" | "finished";
	players: Record<string, PlayerEntry>;
	winnerTeam: number | null;
	winnerPlayerId: string | null;
}

interface AssignedPlayer {
	playerId: string;
	token: string;
	teamId: number;
}

export const arenaMatch = actor({
	options: { name: "Arena - Match", icon: "crosshairs" },
	events: {
		snapshot: event<Snapshot>(),
		shoot: event<ShootEvent>(),
	},
	createState: (
		_c,
		input: {
			matchId: string;
			mode: Mode;
			capacity: number;
			assignedPlayers: AssignedPlayer[];
		},
	): State => {
		const players: Record<string, PlayerEntry> = {};
		for (const ap of input.assignedPlayers) {
			players[ap.playerId] = {
				token: ap.token,
				connId: null,
				teamId: ap.teamId,
				x: Math.random() * WORLD_SIZE,
				y: Math.random() * WORLD_SIZE,
				lastPositionAt: Date.now(),
				alive: true,
				score: 0,
			};
		}
		return {
			matchId: input.matchId,
			mode: input.mode,
			capacity: input.capacity,
			tick: 0,
			phase: "waiting",
			players,
			winnerTeam: null,
			winnerPlayerId: null,
		};
	},
	onBeforeConnect: (
		c,
		params: { playerToken?: string; internalToken?: string },
	) => {
		if (params?.internalToken === INTERNAL_TOKEN) return;
		const playerToken = params?.playerToken?.trim();
		if (!playerToken) {
			throw new UserError("authentication required", {
				code: "auth_required",
			});
		}
		if (!findPlayerByToken(c.state, playerToken)) {
			throw new UserError("invalid player token", {
				code: "invalid_player_token",
			});
		}
	},
	onConnect: (c, conn) => {
		const playerToken = conn.params?.playerToken?.trim();
		if (!playerToken) return;
		const found = findPlayerByToken(c.state, playerToken);
		if (!found) {
			conn.disconnect("invalid_player_token");
			return;
		}
		const [, player] = found;
		player.connId = conn.id;

		// Check if all players have connected → transition to live.
		if (c.state.phase === "waiting") {
			const allConnected = Object.values(c.state.players).every(
				(p) => p.connId !== null,
			);
			if (allConnected) {
				c.state.phase = "live";
			}
		}

		broadcastSnapshot(c);
	},
	onDestroy: async (c) => {
		const client = c.client<typeof registry>();
		await client.arenaMatchmaker
			.getOrCreate(["main"])
			.send("matchCompleted", { matchId: c.state.matchId });
	},
	onDisconnect: (c, conn) => {
		const found = findPlayerByConnId(c.state, conn.id);
		if (!found) return;
		const [, player] = found;
		player.connId = null;
		broadcastSnapshot(c);
	},
	run: async (c) => {
		const tick = interval(TICK_MS);
		while (!c.aborted) {
			await tick();
			if (c.aborted) break;

			if (c.state.phase !== "live") continue;

			c.state.tick += 1;

			checkWinCondition(c);

			broadcastSnapshot(c);
		}
	},
	actions: {
		updatePosition: (c, input: { x: number; y: number }) => {
			const found = findPlayerByConnId(c.state, c.conn.id);
			if (!found) {
				throw new UserError("player not found", {
					code: "player_not_found",
				});
			}
			const [, player] = found;
			if (!player.alive) {
				throw new UserError("player is not alive", { code: "not_alive" });
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
				// Clamp to max allowed distance along the movement vector.
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
			const found = findPlayerByConnId(c.state, c.conn.id);
			if (!found) {
				throw new UserError("player not found", {
					code: "player_not_found",
				});
			}
			const [shooterId, shooter] = found;
			if (!shooter.alive) {
				throw new UserError("player is not alive", { code: "not_alive" });
			}
			if (c.state.phase !== "live") {
				throw new UserError("match is not live", { code: "not_live" });
			}

			// Normalize direction.
			const mag = Math.sqrt(
				input.dirX * input.dirX + input.dirY * input.dirY,
			);
			if (mag === 0) return;
			const ndx = input.dirX / mag;
			const ndy = input.dirY / mag;

			// Find closest valid hit.
			let closestId: string | null = null;
			let closestDist = Infinity;

			for (const [targetId, target] of Object.entries(c.state.players)) {
				if (targetId === shooterId) continue;
				if (!target.alive) continue;
				// In team modes, skip teammates.
				if (shooter.teamId >= 0 && target.teamId === shooter.teamId) continue;

				const tx = target.x - shooter.x;
				const ty = target.y - shooter.y;
				const targetDist = Math.sqrt(tx * tx + ty * ty);
				if (targetDist > SHOOT_RANGE || targetDist === 0) continue;

				// Check angle between shot direction and target direction.
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
				shooter.score += 1;
				// Respawn victim at a random position.
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

			// Check win condition after scoring.
			checkWinCondition(c);
			if ((c.state.phase as string) === "finished") {
				broadcastSnapshot(c);
			}
		},
		getSnapshot: (c) => buildSnapshot(c),
	},
});

function checkWinCondition(c: ActorContextOf<typeof arenaMatch>) {
	if (c.state.phase !== "live") return;

	if (c.state.mode === "ffa") {
		for (const [id, player] of Object.entries(c.state.players)) {
			if (player.score >= SCORE_LIMIT) {
				c.state.phase = "finished";
				c.state.winnerPlayerId = id;
				return;
			}
		}
	} else {
		// Team mode: sum scores per team.
		const teamScores: Record<number, number> = {};
		for (const player of Object.values(c.state.players)) {
			teamScores[player.teamId] = (teamScores[player.teamId] ?? 0) + player.score;
		}
		for (const [teamId, score] of Object.entries(teamScores)) {
			if (score >= SCORE_LIMIT) {
				c.state.phase = "finished";
				c.state.winnerTeam = Number(teamId);
				return;
			}
		}
	}
}

function broadcastSnapshot(c: ActorContextOf<typeof arenaMatch>) {
	c.broadcast("snapshot", buildSnapshot(c));
}

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
	players: Record<string, { x: number; y: number; teamId: number; score: number }>;
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
	const players: Record<string, { x: number; y: number; teamId: number; score: number }> = {};
	for (const [id, entry] of Object.entries(c.state.players)) {
		players[id] = {
			x: entry.x,
			y: entry.y,
			teamId: entry.teamId,
			score: entry.score,
		};
	}
	return {
		matchId: c.state.matchId,
		mode: c.state.mode,
		capacity: c.state.capacity,
		tick: c.state.tick,
		phase: c.state.phase,
		winnerTeam: c.state.winnerTeam,
		winnerPlayerId: c.state.winnerPlayerId,
		worldSize: WORLD_SIZE,
		scoreLimit: SCORE_LIMIT,
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

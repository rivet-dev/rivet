import { actor, type ActorContextOf, event, UserError } from "rivetkit";
import { interval } from "rivetkit/utils";
import {
	hasInvalidInternalToken,
	INTERNAL_TOKEN,
	isInternalToken,
} from "../../auth.ts";
import { registry } from "../index.ts";
import {
	TICK_MS,
	WORLD_SIZE,
	MAX_SPEED,
	SHOOT_RANGE,
	SHOOT_ANGLE,
	SCORE_LIMIT,
	K_FACTOR,
} from "./config.ts";

interface PlayerEntry {
	token: string;
	connId: string | null;
	x: number;
	y: number;
	lastPositionAt: number;
	alive: boolean;
	score: number;
	rating: number;
}

interface State {
	matchId: string;
	tick: number;
	phase: "waiting" | "live" | "finished";
	players: Record<string, PlayerEntry>;
	winnerId: string | null;
}

interface AssignedPlayer {
	username: string;
	rating: number;
	token: string;
}

export const rankedMatch = actor({
	options: { name: "Ranked - Match", icon: "ranking-star" },
	events: {
		snapshot: event<Snapshot>(),
		shoot: event<ShootEvent>(),
	},
	createState: (
		_c,
		input: { matchId: string; assignedPlayers: AssignedPlayer[] },
	): State => {
		const players: Record<string, PlayerEntry> = {};
		for (const ap of input.assignedPlayers) {
			players[ap.username] = {
				token: ap.token,
				connId: null,
				x: Math.random() * WORLD_SIZE,
				y: Math.random() * WORLD_SIZE,
				lastPositionAt: Date.now(),
				alive: true,
				score: 0,
				rating: ap.rating,
			};
		}
		return {
			matchId: input.matchId,
			tick: 0,
			phase: "waiting",
			players,
			winnerId: null,
		};
	},
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
		const entries = Object.entries(c.state.players);
		if (c.state.winnerId && entries.length === 2) {
			const loserUsername = entries.find(([id]) => id !== c.state.winnerId)?.[0];
			const winner = c.state.players[c.state.winnerId];
			const loser = loserUsername ? c.state.players[loserUsername] : undefined;
			if (winner && loser && loserUsername) {
				const [newWR, newLR] = calculateElo(winner.rating, loser.rating);
				await client.rankedMatchmaker
					.getOrCreate(["main"], { params: { internalToken: INTERNAL_TOKEN } })
					.send("matchCompleted", {
						matchId: c.state.matchId,
						winnerUsername: c.state.winnerId,
						loserUsername,
						winnerNewRating: newWR,
						loserNewRating: newLR,
					});
				return;
			}
		}
		await client.rankedMatchmaker
			.getOrCreate(["main"], { params: { internalToken: INTERNAL_TOKEN } })
			.send("matchCompleted", {
				matchId: c.state.matchId,
				winnerUsername: "",
				loserUsername: "",
				winnerNewRating: 0,
				loserNewRating: 0,
			});
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
				throw new UserError("player not found", { code: "player_not_found" });
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
				throw new UserError("player not found", { code: "player_not_found" });
			}
			const [shooterId, shooter] = found;
			if (!shooter.alive) {
				throw new UserError("player is not alive", { code: "not_alive" });
			}
			if (c.state.phase !== "live") {
				throw new UserError("match is not live", { code: "not_live" });
			}

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
			if ((c.state.phase as string) === "finished") {
				broadcastSnapshot(c);
			}
		},
		getSnapshot: (c) => buildSnapshot(c),
	},
});

function checkWinCondition(c: ActorContextOf<typeof rankedMatch>) {
	if (c.state.phase !== "live") return;
	for (const [id, player] of Object.entries(c.state.players)) {
		if (player.score >= SCORE_LIMIT) {
			c.state.phase = "finished";
			c.state.winnerId = id;
			return;
		}
	}
}

function calculateElo(winnerRating: number, loserRating: number): [number, number] {
	const expectedW = 1 / (1 + 10 ** ((loserRating - winnerRating) / 400));
	const expectedL = 1 - expectedW;
	return [
		Math.round(winnerRating + K_FACTOR * (1 - expectedW)),
		Math.round(loserRating + K_FACTOR * (0 - expectedL)),
	];
}

function broadcastSnapshot(c: ActorContextOf<typeof rankedMatch>) {
	c.broadcast("snapshot", buildSnapshot(c));
}

interface Snapshot {
	matchId: string;
	tick: number;
	phase: "waiting" | "live" | "finished";
	winnerId: string | null;
	worldSize: number;
	scoreLimit: number;
	players: Record<string, { x: number; y: number; score: number; rating: number }>;
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
	for (const [id, entry] of Object.entries(c.state.players)) {
		players[id] = {
			x: entry.x,
			y: entry.y,
			score: entry.score,
			rating: entry.rating,
		};
	}
	return {
		matchId: c.state.matchId,
		tick: c.state.tick,
		phase: c.state.phase,
		winnerId: c.state.winnerId,
		worldSize: WORLD_SIZE,
		scoreLimit: SCORE_LIMIT,
		players,
	};
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

function findPlayerByToken(
	state: State,
	token: string,
): [string, PlayerEntry] | null {
	for (const [id, entry] of Object.entries(state.players)) {
		if (entry.token === token) return [id, entry];
	}
	return null;
}

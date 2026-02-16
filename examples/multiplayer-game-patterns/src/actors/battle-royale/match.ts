import { actor } from "rivetkit";
import { err, sleep } from "../../utils.ts";
import { buildSecret } from "../shared/ids.ts";

interface MatchSeat {
	playerId: string;
	name: string;
}

interface Player {
	playerId: string;
	name: string;
	joinedAt: number;
	alive: boolean;
}

interface State {
	matchId: string;
	trusted: boolean;
	tickMs: number;
	seats: MatchSeat[];
	playerIds: string[];
	players: Record<string, Player>;
	playerTokens: Record<string, string>;
	phase: "waiting" | "active" | "finished";
	tick: number;
	zoneRadius: number;
	winnerPlayerId: string | null;
}

function requireTrusted(state: State) {
	if (!state.trusted) {
		err("match is not trusted", "untrusted_lobby");
	}
}

function findSeatByPlayerId(state: State, playerId: string): MatchSeat | null {
	for (const seat of state.seats) {
		if (seat.playerId === playerId) {
			return seat;
		}
	}
	return null;
}

function buildSnapshot(state: State) {
	// Snapshot exposes scaffold battle royale state for UI and tests.
	const alivePlayerIds = Object.values(state.players)
		.filter((player) => player.alive)
		.map((player) => player.playerId);
	return {
		matchId: state.matchId,
		phase: state.phase,
		tick: state.tick,
		zoneRadius: state.zoneRadius,
		winnerPlayerId: state.winnerPlayerId,
		players: state.players,
		alivePlayerIds,
	};
}

function maybeStart(state: State) {
	// Auto-start once all assigned players have joined.
	if (state.phase !== "waiting") return;
	const joined = Object.keys(state.players).length;
	if (joined >= state.playerIds.length) {
		state.phase = "active";
	}
}

async function notifyMatchClosed(c: {
	state: State;
	client: <T>() => any;
}) {
	const client = c.client<any>();
	try {
		await client.battleRoyaleMatchmaker.getOrCreate(["main"]).queue.matchClosed.send({
			matchId: c.state.matchId,
		});
	} catch {
		// Best effort during shutdown.
	}
}

async function maybeFinish(c: {
	state: State;
	broadcast: (event: string, payload: unknown) => void;
	client: <T>() => any;
}) {
	if (c.state.phase !== "active") return;
	const alive = Object.values(c.state.players).filter((player) => player.alive);
	if (alive.length > 1) return;
	c.state.phase = "finished";
	c.state.winnerPlayerId = alive[0]?.playerId ?? null;
	c.broadcast("snapshot", buildSnapshot(c.state));
	await notifyMatchClosed(c);
}

export const battleRoyaleMatch = actor({
	createState: (
		_c,
		input: { matchId: string; tickMs: number; players: MatchSeat[] },
	): State => ({
		matchId: input.matchId,
		trusted: true,
		tickMs: input.tickMs,
		seats: input.players,
		playerIds: input.players.map((player) => player.playerId),
		players: {},
		playerTokens: {},
		phase: "waiting",
		tick: 0,
		zoneRadius: 120,
		winnerPlayerId: null,
	}),
	createConnState: (_c, _params: { playerToken?: string }) => ({
		playerId: null as string | null,
	}),
	onBeforeConnect: (c, params: { playerToken?: string }) => {
		const playerToken = params?.playerToken?.trim();
		if (!playerToken) {
			return;
		}
		const playerId = c.state.playerTokens[playerToken];
		if (!playerId) {
			err("invalid player token", "invalid_player_token");
		}
	},
	onConnect: (c, conn) => {
		const playerToken = conn.params?.playerToken?.trim();
		if (!playerToken) {
			conn.disconnect("invalid_player_token");
			return;
		}
		const playerId = c.state.playerTokens[playerToken];
		if (!playerId) {
			conn.disconnect("invalid_player_token");
			return;
		}
		const seat = findSeatByPlayerId(c.state, playerId);
		if (!seat) {
			conn.disconnect("invalid_player_assignment");
			return;
		}
		if (!c.state.players[seat.playerId]) {
			c.state.players[seat.playerId] = {
				playerId: seat.playerId,
				name: seat.name,
				joinedAt: Date.now(),
				alive: true,
			};
		}
		conn.state.playerId = seat.playerId;
		maybeStart(c.state);
		c.broadcast("snapshot", buildSnapshot(c.state));
	},
	onDisconnect: async (c, conn) => {
		const playerId = conn.state.playerId;
		if (!playerId) return;
		if (!c.state.players[playerId]) return;

		delete c.state.players[playerId];
		conn.state.playerId = null;

		if (Object.keys(c.state.players).length === 0) {
			c.destroy();
			return;
		}

		await maybeFinish(c);
		c.broadcast("snapshot", buildSnapshot(c.state));
	},
	onDestroy: async (c) => {
		await notifyMatchClosed(c);
	},
	run: async (c) => {
		// This run loop is the 10 tps scaffold tick for battle royale sessions.
		while (!c.aborted) {
			await sleep(c.state.tickMs);
			if (c.aborted) break;
			if (c.state.phase !== "active") continue;
			c.state.tick += 1;
			// Simulate shrinking safe zone to demonstrate phase progression.
			c.state.zoneRadius = Math.max(5, c.state.zoneRadius - 0.25);
			await maybeFinish(c);
			c.broadcast("snapshot", buildSnapshot(c.state));
		}
	},
	actions: {
		issuePlayerToken: (
			c,
			input: { playerId: string },
		) => {
			const seat = findSeatByPlayerId(c.state, input.playerId);
			if (!seat) {
				err("player is not assigned", "invalid_player");
			}
			const playerToken = buildSecret();
			c.state.playerTokens[playerToken] = seat.playerId;
			return { playerId: seat.playerId, playerToken };
		},
		startNow: (c) => {
			requireTrusted(c.state);
			if (!c.conn.state.playerId) {
				err("caller is not joined", "not_joined");
			}
			// Manual start is useful for testing with partial lobbies.
			if (c.state.phase === "waiting") {
				c.state.phase = "active";
				c.broadcast("snapshot", buildSnapshot(c.state));
			}
			return buildSnapshot(c.state);
		},
		eliminate: async (c, input: { victimPlayerId: string }) => {
			requireTrusted(c.state);
			if (!c.conn.state.playerId) {
				err("caller is not joined", "not_joined");
			}
			// This action is scaffold logic for elimination and winner resolution.
			if (c.state.phase !== "active") {
				err("match not active", "match_not_active");
			}
			const victim = c.state.players[input.victimPlayerId];
			if (!victim) {
				err("victim not found", "victim_not_found");
			}
			victim.alive = false;
			await maybeFinish(c);
			c.broadcast("snapshot", buildSnapshot(c.state));
			return buildSnapshot(c.state);
		},
		getSnapshot: (c) => buildSnapshot(c.state),
	},
});

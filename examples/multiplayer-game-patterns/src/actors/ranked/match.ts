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
	lastInputAt: number;
}

interface State {
	matchId: string;
	tickMs: number;
	trusted: boolean;
	seats: [MatchSeat, MatchSeat];
	playerIds: [string, string];
	players: Record<string, Player>;
	playerTokens: Record<string, string>;
	phase: "waiting" | "live" | "finished";
	tick: number;
	winnerPlayerId: string | null;
	resultRatings: Record<string, number> | null;
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
	// Ranked snapshot includes match phase, tick, and optional rating result.
	return {
		matchId: state.matchId,
		phase: state.phase,
		tick: state.tick,
		players: state.players,
		playerIds: state.playerIds,
		winnerPlayerId: state.winnerPlayerId,
		resultRatings: state.resultRatings,
	};
}

export const rankedMatch = actor({
	createState: (
		_c,
		input: {
			matchId: string;
			tickMs: number;
			players: [MatchSeat, MatchSeat];
		},
	): State => ({
		matchId: input.matchId,
		tickMs: input.tickMs,
		trusted: true,
		seats: input.players,
		playerIds: [input.players[0].playerId, input.players[1].playerId],
		players: {},
		playerTokens: {},
		phase: "waiting",
		tick: 0,
		winnerPlayerId: null,
		resultRatings: null,
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
				lastInputAt: Date.now(),
			};
		}
		conn.state.playerId = seat.playerId;
		if (Object.keys(c.state.players).length === c.state.playerIds.length) {
			c.state.phase = "live";
		}
		c.broadcast("snapshot", buildSnapshot(c.state));
	},
	onDisconnect: (c, conn) => {
		const playerId = conn.state.playerId;
		if (!playerId) return;
		if (!c.state.players[playerId]) return;

		delete c.state.players[playerId];
		conn.state.playerId = null;

		if (Object.keys(c.state.players).length === 0) {
			c.destroy();
			return;
		}

		if (c.state.phase !== "finished") {
			c.state.phase =
				Object.keys(c.state.players).length === c.state.playerIds.length
					? "live"
					: "waiting";
		}
		c.broadcast("snapshot", buildSnapshot(c.state));
	},
	run: async (c) => {
		// This run loop is the 20 tps scaffold tick for ranked matches.
		while (!c.aborted) {
			await sleep(c.state.tickMs);
			if (c.aborted) break;
			if (c.state.phase !== "live") continue;
			c.state.tick += 1;
			// Broadcast a canonical snapshot every tick while live.
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
		finish: async (c, input: { winnerPlayerId?: string | null }) => {
			requireTrusted(c.state);
			if (!c.conn.state.playerId) {
				err("caller is not joined", "not_joined");
			}
			if (input.winnerPlayerId && !c.state.playerIds.includes(input.winnerPlayerId)) {
				err("winner player is not assigned", "invalid_winner_player");
			}
			if (c.state.phase === "finished") {
				return buildSnapshot(c.state);
			}

			c.state.phase = "finished";
			c.state.winnerPlayerId = input.winnerPlayerId ?? null;
			const client = c.client<any>();
			try {
				// Matchmaker owns SQLite ELO and assignment state.
				// Queue the result report so the matchmaker run loop applies updates.
				await client.rankedMatchmaker.getOrCreate(["main"]).queue.reportResult.send({
					matchId: c.state.matchId,
					winnerPlayerId: c.state.winnerPlayerId,
				});
			} catch {
				// Best effort during shutdown.
			}
			c.broadcast("snapshot", buildSnapshot(c.state));
			return buildSnapshot(c.state);
		},
		getSnapshot: (c) => buildSnapshot(c.state),
	},
});

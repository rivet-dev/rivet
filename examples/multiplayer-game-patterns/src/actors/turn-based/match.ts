import { actor } from "rivetkit";
import { err } from "../../utils.ts";
import { buildSecret } from "../shared/ids.ts";

interface MatchSeat {
	playerId: string;
	name: string;
}

interface TurnPlayer {
	playerId: string;
	name: string;
	joinedAt: number;
}

interface TurnMove {
	turn: number;
	playerId: string;
	move: string;
	at: number;
}

interface State {
	matchId: string;
	trusted: boolean;
	seats: [MatchSeat, MatchSeat];
	playerIds: [string, string];
	phase: "waiting" | "in_progress" | "finished";
	players: Record<string, TurnPlayer>;
	playerTokens: Record<string, string>;
	turnIndex: number;
	moves: TurnMove[];
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
	// The async scaffold snapshot keeps turn order and move history explicit.
	const nextPlayerId =
		state.phase === "in_progress"
			? state.playerIds[state.turnIndex % state.playerIds.length]
			: null;

	return {
		matchId: state.matchId,
		playerIds: state.playerIds,
		phase: state.phase,
		players: state.players,
		turnIndex: state.turnIndex,
		nextPlayerId,
		moves: state.moves,
		winnerPlayerId: state.winnerPlayerId,
	};
}

export const asyncTurnBasedMatch = actor({
	createState: (
		_c,
		input: {
			matchId: string;
			players: [MatchSeat, MatchSeat];
		},
	): State => ({
		matchId: input.matchId,
		trusted: true,
		seats: input.players,
		playerIds: [input.players[0].playerId, input.players[1].playerId],
		phase: "waiting",
		players: {},
		playerTokens: {},
		turnIndex: 0,
		moves: [],
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
			};
		}
		conn.state.playerId = seat.playerId;

		if (Object.keys(c.state.players).length === c.state.playerIds.length) {
			c.state.phase = "in_progress";
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
					? "in_progress"
					: "waiting";
		}
		c.broadcast("snapshot", buildSnapshot(c.state));
	},
	onDestroy: async (c) => {
		const client = c.client<any>();
		try {
			// Matchmaker assignment rows live in SQLite.
			// This callback clears those rows when the match actor shuts down.
			await client.asyncTurnBasedMatchmaker.getOrCreate(["main"]).queue.matchCompleted.send({
				matchId: c.state.matchId,
			});
		} catch {
			// Best effort during shutdown.
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
		submitTurn: (c, input: { move: string }) => {
			requireTrusted(c.state);
			const playerId = c.conn.state.playerId;
			if (!playerId) {
				err("caller is not joined", "not_joined");
			}
			// Turn submission validates phase, ownership, and strict turn order.
			if (c.state.phase !== "in_progress") {
				err("match is not in progress", "match_not_live");
			}
			const expected = c.state.playerIds[c.state.turnIndex % c.state.playerIds.length];
			if (expected !== playerId) {
				err("not your turn", "not_your_turn");
			}

			const move: TurnMove = {
				turn: c.state.turnIndex,
				playerId,
				move: input.move,
				at: Date.now(),
			};
			c.state.moves.push(move);
			c.state.turnIndex += 1;
			const nextPlayerId = c.state.playerIds[c.state.turnIndex % c.state.playerIds.length];
			// Emit both the committed move and the resulting snapshot.
			c.broadcast("turnCommitted", { move, nextPlayerId });
			c.broadcast("snapshot", buildSnapshot(c.state));
			return { ok: true, nextPlayerId };
		},
		finish: async (c, input: { winnerPlayerId?: string | null }) => {
			requireTrusted(c.state);
			if (!c.conn.state.playerId) {
				err("caller is not joined", "not_joined");
			}
			if (input.winnerPlayerId && !c.state.playerIds.includes(input.winnerPlayerId)) {
				err("winner player is not assigned", "invalid_winner_player");
			}
			c.state.phase = "finished";
			c.state.winnerPlayerId = input.winnerPlayerId ?? null;
			c.broadcast("snapshot", buildSnapshot(c.state));

			const client = c.client<any>();
			// Keep matchmaker assignment state in sync with the finished phase.
			await client.asyncTurnBasedMatchmaker.getOrCreate(["main"]).queue.matchCompleted.send({
				matchId: c.state.matchId,
			});

			return buildSnapshot(c.state);
		},
		getSnapshot: (c) => buildSnapshot(c.state),
	},
});

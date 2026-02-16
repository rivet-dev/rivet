import { actor } from "rivetkit";
import { err } from "../../utils.ts";
import { buildSecret } from "../shared/ids.ts";

interface Player {
	playerId: string;
	name: string;
	joinedAt: number;
}

interface State {
	matchId: string;
	partyCode: string;
	hostPlayerId: string;
	trusted: boolean;
	phase: "lobby" | "in_progress" | "finished";
	players: Record<string, Player>;
	playerTokens: Record<string, string>;
	startedAt: number | null;
}

function requireTrusted(state: State) {
	if (!state.trusted) {
		err("match is not trusted", "untrusted_lobby");
	}
}

function requireJoinedHost(c: {
	state: State;
	conn: { state: { playerId: string | null } };
}) {
	const playerId = c.conn.state.playerId;
	if (!playerId) {
		err("caller is not joined", "not_joined");
	}
	if (playerId !== c.state.hostPlayerId) {
		err("only host can perform this action", "host_only");
	}
}

function buildSnapshot(state: State) {
	// Party snapshot favors readability over game-specific detail.
	return {
		matchId: state.matchId,
		partyCode: state.partyCode,
		hostPlayerId: state.hostPlayerId,
		phase: state.phase,
		players: state.players,
		startedAt: state.startedAt,
	};
}

async function closeLobby(c: {
	state: State;
	client: <T>() => any;
}) {
	const client = c.client<any>();
	try {
		// Party discovery lives in matchmaker SQLite tables.
		// This call removes party rows when the match actor shuts down.
		await client.partyMatchmaker.getOrCreate(["main"]).queue.closeParty.send({
			partyCode: c.state.partyCode,
		});
	} catch {
		// Best effort during shutdown.
	}
}

export const partyMatch = actor({
	createState: (
		_c,
		input: { matchId: string; partyCode: string; hostPlayerId: string },
	): State => ({
		matchId: input.matchId,
		partyCode: input.partyCode,
		hostPlayerId: input.hostPlayerId,
		trusted: true,
		phase: "lobby",
		players: {},
		playerTokens: {},
		startedAt: null,
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
		if (c.state.phase !== "lobby" && !c.state.players[playerId]) {
			err("party is no longer joinable", "party_closed");
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
		if (c.state.phase !== "lobby" && !c.state.players[playerId]) {
			conn.disconnect("party_closed");
			return;
		}
		if (!c.state.players[playerId]) {
			c.state.players[playerId] = {
				playerId,
				name: playerId,
				joinedAt: Date.now(),
			};
		}
		conn.state.playerId = playerId;
		c.broadcast("snapshot", buildSnapshot(c.state));
	},
	onDisconnect: (c, conn) => {
		const playerId = conn.state.playerId;
		if (!playerId) return;
		// This scaffold treats disconnect as leaving the party.
		delete c.state.players[playerId];
		conn.state.playerId = null;
		c.broadcast("snapshot", buildSnapshot(c.state));
		if (Object.keys(c.state.players).length === 0) {
			c.destroy();
		}
	},
	onDestroy: async (c) => {
		await closeLobby(c);
	},
	actions: {
		issuePlayerToken: (
			c,
			input: { playerId: string },
		) => {
			const playerToken = buildSecret();
			c.state.playerTokens[playerToken] = input.playerId;
			return { playerId: input.playerId, playerToken };
		},
		start: async (c) => {
			requireTrusted(c.state);
			requireJoinedHost(c);
			if (Object.keys(c.state.players).length < 2) {
				err("need at least two players", "not_enough_players");
			}
			if (c.state.phase !== "lobby") {
				err("party already started", "already_started");
			}

			c.state.phase = "in_progress";
			c.state.startedAt = Date.now();
			c.broadcast("snapshot", buildSnapshot(c.state));

			const client = c.client<any>();
			// Persist the phase transition in the SQLite party index.
			await client.partyMatchmaker.getOrCreate(["main"]).queue.markStarted.send({
				partyCode: c.state.partyCode,
			});

			return buildSnapshot(c.state);
		},
		finish: async (c) => {
			requireTrusted(c.state);
			requireJoinedHost(c);
			c.state.phase = "finished";
			c.broadcast("snapshot", buildSnapshot(c.state));
			return buildSnapshot(c.state);
		},
		getSnapshot: (c) => buildSnapshot(c.state),
	},
});

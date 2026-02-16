import { actor } from "rivetkit";
import { err, sleep } from "../../utils.ts";
import { buildSecret } from "../shared/ids.ts";
import type { Mode } from "./matchmaker.ts";

interface AssignedPlayer {
	playerId: string;
	name: string;
	teamId: number;
}

interface Player {
	playerId: string;
	name: string;
	teamId: number;
	joinedAt: number;
	lastActionAt: number;
}

interface State {
	matchId: string;
	trusted: boolean;
	mode: Mode;
	capacity: number;
	tickMs: number;
	tick: number;
	phase: "waiting" | "live" | "finished";
	assignedPlayers: AssignedPlayer[];
	players: Record<string, Player>;
	playerTokens: Record<string, string>;
	winnerTeam: number | null;
}

function requireTrusted(state: State) {
	if (!state.trusted) {
		err("match is not trusted", "untrusted_lobby");
	}
}

function findAssignedByPlayerId(state: State, playerId: string) {
	return state.assignedPlayers.find((entry) => entry.playerId === playerId) ?? null;
}

function maxTeamId(state: State): number {
	let max = 0;
	for (const assigned of state.assignedPlayers) {
		if (assigned.teamId > max) {
			max = assigned.teamId;
		}
	}
	return max;
}

function buildSnapshot(state: State) {
	// This snapshot shape is intentionally simple for frontend and test harness use.
	return {
		matchId: state.matchId,
		mode: state.mode,
		capacity: state.capacity,
		tick: state.tick,
		phase: state.phase,
		winnerTeam: state.winnerTeam,
		assignedPlayers: state.assignedPlayers.map((entry) => ({
			playerId: entry.playerId,
			teamId: entry.teamId,
		})),
		players: state.players,
		joinedCount: Object.keys(state.players).length,
	};
}

export const competitiveMatch = actor({
	createState: (
		_c,
		input: {
			matchId: string;
			mode: Mode;
			capacity: number;
			tickMs: number;
			assignedPlayers: AssignedPlayer[];
		},
	): State => ({
		matchId: input.matchId,
		trusted: true,
		mode: input.mode,
		capacity: input.capacity,
		tickMs: input.tickMs,
		tick: 0,
		phase: "waiting",
		assignedPlayers: input.assignedPlayers,
		players: {},
		playerTokens: {},
		winnerTeam: null,
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
		const assigned = findAssignedByPlayerId(c.state, playerId);
		if (!assigned) {
			conn.disconnect("invalid_player_assignment");
			return;
		}

		const existing = c.state.players[assigned.playerId];
		if (!existing) {
			c.state.players[assigned.playerId] = {
				playerId: assigned.playerId,
				name: assigned.name,
				teamId: assigned.teamId,
				joinedAt: Date.now(),
				lastActionAt: Date.now(),
			};
		}

		conn.state.playerId = assigned.playerId;
		if (Object.keys(c.state.players).length === c.state.assignedPlayers.length) {
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
				Object.keys(c.state.players).length === c.state.assignedPlayers.length
					? "live"
					: "waiting";
		}
		c.broadcast("snapshot", buildSnapshot(c.state));
	},
	onDestroy: async (c) => {
		const client = c.client<any>();
		try {
			// The matchmaker persists competitive assignments in SQLite.
			// This callback clears those rows when the match actor goes away.
			await client.competitiveMatchmaker.getOrCreate(["main"]).queue.matchCompleted.send({
				matchId: c.state.matchId,
			});
		} catch {
			// Best effort during shutdown.
		}
	},
	run: async (c) => {
		// This run loop is the 20 tps scaffold tick for competitive sessions.
		while (!c.aborted) {
			await sleep(c.state.tickMs);
			if (c.aborted) break;
			if (c.state.phase !== "live") continue;
			c.state.tick += 1;
			// Broadcast one canonical state update per tick.
			c.broadcast("snapshot", buildSnapshot(c.state));
		}
	},
	actions: {
		issuePlayerToken: (
			c,
			input: { playerId: string },
		) => {
			const assigned = findAssignedByPlayerId(c.state, input.playerId);
			if (!assigned) {
				err("player is not assigned", "invalid_player");
			}
			const playerToken = buildSecret();
			c.state.playerTokens[playerToken] = assigned.playerId;
			return { playerId: assigned.playerId, teamId: assigned.teamId, playerToken };
		},
		finish: async (c, input: { winnerTeam: number | null }) => {
			requireTrusted(c.state);
			if (!c.conn.state.playerId) {
				err("caller is not joined", "not_joined");
			}
			if (input.winnerTeam != null) {
				if (input.winnerTeam < 0 || input.winnerTeam > maxTeamId(c.state)) {
					err("winner team is invalid", "invalid_winner_team");
				}
			}
			c.state.phase = "finished";
			c.state.winnerTeam = input.winnerTeam;
			c.broadcast("snapshot", buildSnapshot(c.state));

			const client = c.client<any>();
			// This keeps SQLite assignment rows in sync with match lifecycle.
			await client.competitiveMatchmaker.getOrCreate(["main"]).queue.matchCompleted.send({
				matchId: c.state.matchId,
			});

			return buildSnapshot(c.state);
		},
		getSnapshot: (c) => buildSnapshot(c.state),
	},
});

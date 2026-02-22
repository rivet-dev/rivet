import { actor, type ActorContextOf, event, UserError } from "rivetkit";
import { registry } from "../index.ts";
import { getPlayerColor } from "../player-color.ts";
import { MAX_PARTY_SIZE, type PartyPhase } from "./config.ts";

interface PartyConnState {
	playerId: string;
	playerName: string;
	isHost: boolean;
}

interface MemberEntry {
	connId: string;
	name: string;
	color: string;
	isHost: boolean;
	isReady: boolean;
}

interface State {
	matchId: string;
	partyCode: string;
	phase: PartyPhase;
	members: Record<string, MemberEntry>;
}

export const partyMatch = actor({
	options: { name: "Party - Match", icon: "people-group" },
	events: {
		partyUpdate: event<PartySnapshot>(),
	},
	createState: (
		_c,
		input: { matchId: string; partyCode: string; hostPlayerId: string },
	): State => ({
		matchId: input.matchId,
		partyCode: input.partyCode,
		phase: "waiting",
		members: {},
	}),
	createConnState: async (
		c,
		params: { playerId?: string; joinToken?: string },
	): Promise<PartyConnState> => {
		const playerId = params?.playerId;
		const joinToken = params?.joinToken;
		const matchId = c.key[0];

		if (!matchId || !playerId || !joinToken) {
			throw new UserError("invalid join params", { code: "invalid_join_params" });
		}

		const client = c.client<typeof registry>();
		const result = await client.partyMatchmaker
			.getOrCreate(["main"])
			.send(
				"verifyJoin",
				{
					matchId,
					playerId,
					joinToken,
				},
				{ wait: true, timeout: 3_000 },
			);
		if (result.status !== "completed") {
			throw new UserError("join verification timed out", {
				code: "join_verification_timed_out",
			});
		}
		const response = (result as {
			response?: { allowed?: boolean; playerName?: string; isHost?: boolean };
		}).response;
		if (!response?.allowed || !response.playerName) {
			throw new UserError("invalid join ticket", { code: "invalid_join_ticket" });
		}

		return {
			playerId,
			playerName: response.playerName,
			isHost: response.isHost === true,
		};
	},
	onConnect: async (c, conn) => {
		const playerId = conn.state.playerId;
		const existing = c.state.members[playerId];
		if (!existing && Object.keys(c.state.members).length >= MAX_PARTY_SIZE) {
			conn.disconnect("party_full");
			return;
		}
		if (existing && existing.connId !== conn.id) {
			conn.disconnect("duplicate_player");
			return;
		}

		const hasHost = Object.values(c.state.members).some((member) => member.isHost);
		const isHost = conn.state.isHost || !hasHost;
		if (isHost) {
			for (const member of Object.values(c.state.members)) {
				member.isHost = false;
			}
		}

		c.state.members[playerId] = {
			connId: conn.id,
			name: conn.state.playerName,
			color: existing?.color ?? getPlayerColor(playerId),
			isHost,
			isReady: existing?.isReady ?? false,
		};

		broadcastSnapshot(c);
		await updatePartySize(c);
	},
	onDisconnect: async (c, conn) => {
		const playerId = conn.state.playerId;
		const member = c.state.members[playerId];
		if (!member || member.connId !== conn.id) return;

		const removedHost = member.isHost;
		delete c.state.members[playerId];

		if (removedHost) {
			promoteNextHost(c.state);
		}

		broadcastSnapshot(c);
		await updatePartySize(c);
	},
	onDestroy: async (c) => {
		const client = c.client<typeof registry>();
		await client.partyMatchmaker
			.getOrCreate(["main"])
			.send("closeParty", { matchId: c.state.matchId });
	},
	actions: {
		setName: (c, input: { name: string }) => {
			const member = c.state.members[c.conn.state.playerId];
			if (!member || member.connId !== c.conn.id) {
				throw new UserError("member not found", { code: "member_not_found" });
			}
			member.name = input.name.trim() || "Player";
			broadcastSnapshot(c);
		},
		toggleReady: (c) => {
			const member = c.state.members[c.conn.state.playerId];
			if (!member || member.connId !== c.conn.id) {
				throw new UserError("member not found", { code: "member_not_found" });
			}
			member.isReady = !member.isReady;
			broadcastSnapshot(c);
		},
		startGame: (c) => {
			const member = c.state.members[c.conn.state.playerId];
			if (!member || member.connId !== c.conn.id) {
				throw new UserError("member not found", { code: "member_not_found" });
			}
			if (!member.isHost) {
				throw new UserError("only host can start", { code: "not_host" });
			}
			if (c.state.phase !== "waiting") {
				throw new UserError("game already started", { code: "already_started" });
			}
			c.state.phase = "playing";
			broadcastSnapshot(c);
		},
		finishGame: (c) => {
			const member = c.state.members[c.conn.state.playerId];
			if (!member || member.connId !== c.conn.id) {
				throw new UserError("member not found", { code: "member_not_found" });
			}
			if (!member.isHost) {
				throw new UserError("only host can finish", { code: "not_host" });
			}
			if (c.state.phase !== "playing") {
				throw new UserError("game not in progress", { code: "not_playing" });
			}
			c.state.phase = "finished";
			broadcastSnapshot(c);
		},
		getSnapshot: (c) => buildSnapshot(c),
	},
});

interface PartySnapshot {
	matchId: string;
	partyCode: string;
	phase: PartyPhase;
	members: Record<string, { name: string; color: string; isHost: boolean; isReady: boolean; connected: boolean }>;
}

function buildSnapshot(c: ActorContextOf<typeof partyMatch>): PartySnapshot {
	const members: PartySnapshot["members"] = {};
	for (const [id, entry] of Object.entries(c.state.members)) {
		members[id] = {
			name: entry.name,
			color: entry.color,
			isHost: entry.isHost,
			isReady: entry.isReady,
			connected: true,
		};
	}
	return {
		matchId: c.state.matchId,
		partyCode: c.state.partyCode,
		phase: c.state.phase,
		members,
	};
}

function broadcastSnapshot(c: ActorContextOf<typeof partyMatch>) {
	c.broadcast("partyUpdate", buildSnapshot(c));
}

async function updatePartySize(c: ActorContextOf<typeof partyMatch>) {
	const client = c.client<typeof registry>();
	await client.partyMatchmaker
		.getOrCreate(["main"])
		.send("updatePartySize", {
			matchId: c.state.matchId,
			playerCount: Object.keys(c.state.members).length,
		});
}

function promoteNextHost(state: State) {
	for (const member of Object.values(state.members)) {
		member.isHost = false;
	}
	const next = Object.values(state.members)[0];
	if (next) {
		next.isHost = true;
		next.isReady = false;
	}
}

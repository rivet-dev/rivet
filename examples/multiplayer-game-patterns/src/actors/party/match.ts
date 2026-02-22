import { actor, type ActorContextOf, event, UserError } from "rivetkit";
import { INTERNAL_TOKEN } from "../../auth.ts";
import { registry } from "../index.ts";
import type { PartyPhase } from "./config.ts";

interface MemberEntry {
	token: string;
	connId: string | null;
	name: string;
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
	createState: (_c, input: { matchId: string; partyCode: string }): State => ({
		matchId: input.matchId,
		partyCode: input.partyCode,
		phase: "waiting",
		members: {},
	}),
	onBeforeConnect: (
		c,
		params: { playerToken?: string; internalToken?: string },
	) => {
		if (params?.internalToken === INTERNAL_TOKEN) return;
		const playerToken = params?.playerToken?.trim();
		if (!playerToken) {
			throw new UserError("authentication required", { code: "auth_required" });
		}
		if (!findMemberByToken(c.state, playerToken)) {
			throw new UserError("invalid player token", { code: "invalid_player_token" });
		}
	},
	onConnect: (c, conn) => {
		const playerToken = conn.params?.playerToken?.trim();
		if (!playerToken) return;
		const found = findMemberByToken(c.state, playerToken);
		if (!found) {
			conn.disconnect("invalid_player_token");
			return;
		}
		const [, member] = found;
		member.connId = conn.id;
		broadcastSnapshot(c);
	},
	onDisconnect: (c, conn) => {
		const found = findMemberByConnId(c.state, conn.id);
		if (!found) return;
		const [, member] = found;
		member.connId = null;
		broadcastSnapshot(c);
	},
	onDestroy: async (c) => {
		const client = c.client<typeof registry>();
		await client.partyMatchmaker
			.getOrCreate(["main"])
			.send("closeParty", { matchId: c.state.matchId });
	},
	actions: {
		createPlayer: (
			c,
			input: { playerId: string; playerToken: string; playerName: string; isHost: boolean },
		) => {
			c.state.members[input.playerId] = {
				token: input.playerToken,
				connId: null,
				name: input.playerName,
				isHost: input.isHost,
				isReady: false,
			};
		},
		setName: (c, input: { name: string }) => {
			const found = findMemberByConnId(c.state, c.conn.id);
			if (!found) {
				throw new UserError("member not found", { code: "member_not_found" });
			}
			const [, member] = found;
			member.name = input.name.trim() || "Player";
			broadcastSnapshot(c);
		},
		toggleReady: (c) => {
			const found = findMemberByConnId(c.state, c.conn.id);
			if (!found) {
				throw new UserError("member not found", { code: "member_not_found" });
			}
			const [, member] = found;
			member.isReady = !member.isReady;
			broadcastSnapshot(c);
		},
		startGame: (c) => {
			const found = findMemberByConnId(c.state, c.conn.id);
			if (!found) {
				throw new UserError("member not found", { code: "member_not_found" });
			}
			const [, member] = found;
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
			const found = findMemberByConnId(c.state, c.conn.id);
			if (!found) {
				throw new UserError("member not found", { code: "member_not_found" });
			}
			const [, member] = found;
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
	members: Record<string, { name: string; isHost: boolean; isReady: boolean; connected: boolean }>;
}

function buildSnapshot(c: ActorContextOf<typeof partyMatch>): PartySnapshot {
	const members: PartySnapshot["members"] = {};
	for (const [id, entry] of Object.entries(c.state.members)) {
		members[id] = {
			name: entry.name,
			isHost: entry.isHost,
			isReady: entry.isReady,
			connected: entry.connId !== null,
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

function findMemberByToken(
	state: State,
	token: string,
): [string, MemberEntry] | null {
	for (const [id, entry] of Object.entries(state.members)) {
		if (entry.token === token) return [id, entry];
	}
	return null;
}

function findMemberByConnId(
	state: State,
	connId: string,
): [string, MemberEntry] | null {
	for (const [id, entry] of Object.entries(state.members)) {
		if (entry.connId === connId) return [id, entry];
	}
	return null;
}

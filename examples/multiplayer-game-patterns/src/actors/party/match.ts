import {
	type ActionContextOf,
	type ActorContextOf,
	actor,
	event,
	UserError,
} from "rivetkit";
import { db } from "rivetkit/db";
import type { registry } from "../index.ts";
import { getPlayerColor } from "../player-color.ts";
import { MAX_PARTY_SIZE, type PartyPhase } from "./config.ts";

interface VerifiedPlayer {
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

interface PartyVars {
	// Connected members, keyed by player id.
	members: Record<string, MemberEntry>;
	// Tickets the matchmaker accepted, keyed by player id.
	verifiedTickets: Map<string, VerifiedPlayer>;
	// The verified player behind each live connection, keyed by connection id.
	players: Map<string, VerifiedPlayer>;
}

export const partyMatch = actor({
	options: { name: "Party - Match", icon: "people-group" },
	events: {
		partyUpdate: event<PartySnapshot>(),
	},
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS party (
					id INTEGER PRIMARY KEY CHECK (id = 1),
					party_code TEXT NOT NULL,
					phase TEXT NOT NULL
				)
			`);
		},
	}),
	// The party code and phase are saved in SQLite, so they survive sleep.
	onCreate: async (c, input: { partyCode: string }) => {
		await c.db.execute(
			"INSERT INTO party (id, party_code, phase) VALUES (1, ?, 'waiting')",
			input.partyCode,
		);
	},
	// Members belong to live connections, which end when the Actor stops, so
	// they only live in memory.
	createVars: (): PartyVars => ({
		members: {},
		verifiedTickets: new Map(),
		players: new Map(),
	}),
	// docs:start connect
	onBeforeConnect: async (
		c,
		params: { playerId?: string; joinToken?: string },
	) => {
		const playerId = params?.playerId;
		const joinToken = params?.joinToken;
		const matchId = c.key[0];

		if (!matchId || !playerId || !joinToken) {
			throw new UserError("invalid join params", {
				code: "invalid_join_params",
			});
		}

		const client = c.client<typeof registry>();
		const ticket = await client.partyMatchmaker
			.getOrCreate(["main"])
			.verifyJoin({ matchId, playerId, joinToken });
		if (!ticket.allowed || !ticket.playerName) {
			throw new UserError("invalid join ticket", {
				code: "invalid_join_ticket",
			});
		}

		c.vars.verifiedTickets.set(playerId, {
			playerId,
			playerName: ticket.playerName,
			isHost: ticket.isHost,
		});
	},
	onConnect: async (c, conn) => {
		const playerId = conn.params?.playerId;
		const verified = playerId
			? c.vars.verifiedTickets.get(playerId)
			: undefined;
		if (!playerId || !verified) {
			conn.disconnect("invalid_join_ticket");
			return;
		}

		const existing = c.vars.members[playerId];
		if (
			!existing &&
			Object.keys(c.vars.members).length >= MAX_PARTY_SIZE
		) {
			conn.disconnect("party_full");
			return;
		}
		if (existing && existing.connId !== conn.id) {
			conn.disconnect("duplicate_player");
			return;
		}

		const hasHost = Object.values(c.vars.members).some(
			(member) => member.isHost,
		);
		const isHost = verified.isHost || !hasHost;
		if (isHost) {
			for (const member of Object.values(c.vars.members)) {
				member.isHost = false;
			}
		}

		c.vars.members[playerId] = {
			connId: conn.id,
			name: verified.playerName,
			color: existing?.color ?? getPlayerColor(playerId),
			isHost,
			isReady: existing?.isReady ?? false,
		};
		c.vars.players.set(conn.id, verified);

		await broadcastSnapshot(c);
		await updatePartySize(c);
	},
	// docs:end connect
	// docs:start host
	onDisconnect: async (c, conn) => {
		const player = c.vars.players.get(conn.id);
		if (!player) return;
		c.vars.players.delete(conn.id);

		const playerId = player.playerId;
		const member = c.vars.members[playerId];
		if (!member || member.connId !== conn.id) return;

		const removedHost = member.isHost;
		delete c.vars.members[playerId];

		if (removedHost) {
			promoteNextHost(c.vars);
		}

		await broadcastSnapshot(c);
		await updatePartySize(c);
	},
	// docs:end host
	onDestroy: async (c) => {
		const client = c.client<typeof registry>();
		await client.partyMatchmaker
			.getOrCreate(["main"])
			.closeParty({ matchId: c.key[0]! });
	},
	actions: {
		setName: async (c, input: { name: string }) => {
			const member = getCallerMember(c);
			if (!member || member.connId !== c.conn.id) {
				throw new UserError("member not found", {
					code: "member_not_found",
				});
			}
			member.name = input.name.trim() || "Player";
			await broadcastSnapshot(c);
		},
		// docs:start host
		toggleReady: async (c) => {
			const member = getCallerMember(c);
			if (!member || member.connId !== c.conn.id) {
				throw new UserError("member not found", {
					code: "member_not_found",
				});
			}
			member.isReady = !member.isReady;
			await broadcastSnapshot(c);
		},
		startGame: async (c) => {
			const member = getCallerMember(c);
			if (!member || member.connId !== c.conn.id) {
				throw new UserError("member not found", {
					code: "member_not_found",
				});
			}
			if (!member.isHost) {
				throw new UserError("only host can start", {
					code: "not_host",
				});
			}
			// The UPDATE only matches in the expected phase, so a second click
			// changes nothing.
			const started = await c.db.execute(
				"UPDATE party SET phase = 'playing' WHERE id = 1 AND phase = 'waiting' RETURNING phase",
			);
			if (started.length === 0) {
				throw new UserError("game already started", {
					code: "already_started",
				});
			}
			await broadcastSnapshot(c);
		},
		finishGame: async (c) => {
			const member = getCallerMember(c);
			if (!member || member.connId !== c.conn.id) {
				throw new UserError("member not found", {
					code: "member_not_found",
				});
			}
			if (!member.isHost) {
				throw new UserError("only host can finish", {
					code: "not_host",
				});
			}
			const finished = await c.db.execute(
				"UPDATE party SET phase = 'finished' WHERE id = 1 AND phase = 'playing' RETURNING phase",
			);
			if (finished.length === 0) {
				throw new UserError("game not in progress", {
					code: "not_playing",
				});
			}
			await broadcastSnapshot(c);
		},
		// docs:end host
		getSnapshot: (c): Promise<PartySnapshot> => buildSnapshot(c),
	},
});

interface PartySnapshot {
	matchId: string;
	partyCode: string;
	phase: PartyPhase;
	members: Record<
		string,
		{
			name: string;
			color: string;
			isHost: boolean;
			isReady: boolean;
			connected: boolean;
		}
	>;
}

async function buildSnapshot(
	c: ActorContextOf<typeof partyMatch>,
): Promise<PartySnapshot> {
	const [party] = await c.db.execute<{
		party_code: string;
		phase: PartyPhase;
	}>("SELECT party_code, phase FROM party WHERE id = 1");

	const members: PartySnapshot["members"] = {};
	for (const [id, entry] of Object.entries(c.vars.members)) {
		members[id] = {
			name: entry.name,
			color: entry.color,
			isHost: entry.isHost,
			isReady: entry.isReady,
			connected: true,
		};
	}
	return {
		matchId: c.key[0]!,
		partyCode: party!.party_code,
		phase: party!.phase,
		members,
	};
}

function getCallerMember(
	c: ActionContextOf<typeof partyMatch>,
): MemberEntry | undefined {
	const player = c.vars.players.get(c.conn.id);
	return player ? c.vars.members[player.playerId] : undefined;
}

async function broadcastSnapshot(c: ActorContextOf<typeof partyMatch>) {
	c.broadcast("partyUpdate", await buildSnapshot(c));
}

async function updatePartySize(c: ActorContextOf<typeof partyMatch>) {
	const client = c.client<typeof registry>();
	await client.partyMatchmaker.getOrCreate(["main"]).updatePartySize({
		matchId: c.key[0]!,
		playerCount: Object.keys(c.vars.members).length,
	});
}

// docs:start host
function promoteNextHost(vars: PartyVars) {
	for (const member of Object.values(vars.members)) {
		member.isHost = false;
	}
	const next = Object.values(vars.members)[0];
	if (next) {
		next.isHost = true;
		next.isReady = false;
	}
}
// docs:end host

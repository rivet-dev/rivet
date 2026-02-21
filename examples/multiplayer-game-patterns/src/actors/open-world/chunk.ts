import { actor, type ActorContextOf, event, UserError } from "rivetkit";
import { interval } from "rivetkit/utils";
import {
	hasInvalidInternalToken,
	INTERNAL_TOKEN,
	isInternalToken,
} from "../../auth.ts";
import { CHUNK_SIZE, TICK_MS, SPEED, SPRINT_MULTIPLIER } from "./config.ts";

const DISCONNECT_GRACE_MS = 5000;
const GRID_COLS = Math.floor(CHUNK_SIZE / 50);

interface PlayerEntry {
	token: string;
	connId: string | null;
	name: string;
	x: number;
	y: number;
	inputX: number;
	inputY: number;
	sprint: boolean;
	disconnectedAt: number | null;
}

interface State {
	worldId: string;
	chunkX: number;
	chunkY: number;
	tick: number;
	players: Record<string, PlayerEntry>;
	blocks: string[];
	initialized: boolean;
}

export const openWorldChunk = actor({
	options: { name: "Open World - Chunk", icon: "map" },
	events: {
		snapshot: event<Snapshot>(),
	},
	state: {
		worldId: "",
		chunkX: 0,
		chunkY: 0,
		tick: 0,
		players: {} as Record<string, PlayerEntry>,
		blocks: [] as string[],
		initialized: false as boolean,
	} satisfies State,
	onBeforeConnect: (
		c,
		params: { playerToken?: string; internalToken?: string; observer?: string },
	) => {
		if (hasInvalidInternalToken(params)) {
			throw new UserError("forbidden", { code: "forbidden" });
		}
		if (params?.internalToken === INTERNAL_TOKEN) return;
		// Allow observer connections (for viewing adjacent chunks).
		if (params?.observer === "true") return;
		const playerToken = params?.playerToken?.trim();
		if (!playerToken) {
			throw new UserError("authentication required", { code: "auth_required" });
		}
		if (!findPlayerByToken(c.state, playerToken)) {
			throw new UserError("invalid player token", { code: "invalid_player_token" });
		}
	},
	canInvoke: (c, invoke) => {
		const params = c.conn.params as
			| { internalToken?: string; observer?: string }
			| undefined;
		const isInternal = isInternalToken(params);
		const isObserver = params?.observer === "true";
		const isAssignedPlayer = findPlayerByConnId(c.state, c.conn.id) !== null;
		if (
			invoke.kind === "action" &&
			(invoke.name === "initialize" || invoke.name === "createPlayer")
		) {
			return isInternal;
		}
		if (invoke.kind === "action" && invoke.name === "getSnapshot") {
			return isObserver || isAssignedPlayer;
		}
		if (
			invoke.kind === "action" &&
			(invoke.name === "setInput" ||
				invoke.name === "placeBlock" ||
				invoke.name === "removeBlock")
		) {
			return isAssignedPlayer;
		}
		if (invoke.kind === "action" && invoke.name === "removePlayer") {
			return isAssignedPlayer || isInternal;
		}
		if (invoke.kind === "subscribe" && invoke.name === "snapshot") {
			return isObserver || isAssignedPlayer;
		}
		return false;
	},
	onConnect: (c, conn) => {
		// Observer connections just receive broadcasts.
		if (conn.params?.observer === "true") return;
		const playerToken = conn.params?.playerToken?.trim();
		if (!playerToken) return;
		const found = findPlayerByToken(c.state, playerToken);
		if (!found) {
			conn.disconnect("invalid_player_token");
			return;
		}
		const [, player] = found;
		player.connId = conn.id;
		player.disconnectedAt = null;
		broadcastSnapshot(c);
	},
	onDisconnect: (c, conn) => {
		const found = findPlayerByConnId(c.state, conn.id);
		if (!found) return;
		const [, player] = found;
		player.connId = null;
		player.disconnectedAt = Date.now();
		broadcastSnapshot(c);
	},
	run: async (c) => {
		if (!c.state.blocks) c.state.blocks = [];
		const tick = interval(TICK_MS);
		while (!c.aborted) {
			await tick();
			if (c.aborted) break;
			c.state.tick += 1;

			const now = Date.now();

			for (const [id, player] of Object.entries(c.state.players)) {
				// Remove disconnected beyond grace.
				if (
					player.disconnectedAt &&
					now - player.disconnectedAt > DISCONNECT_GRACE_MS
				) {
					delete c.state.players[id];
					continue;
				}

				const speed = player.sprint ? SPEED * SPRINT_MULTIPLIER : SPEED;
				player.x += speed * player.inputX;
				player.y += speed * player.inputY;

				// Clamp to chunk bounds.
				player.x = Math.max(0, Math.min(CHUNK_SIZE - 1, player.x));
				player.y = Math.max(0, Math.min(CHUNK_SIZE - 1, player.y));
			}

			broadcastSnapshot(c);
		}
	},
	actions: {
		initialize: (c, input: { worldId: string; chunkX: number; chunkY: number }) => {
			if (c.state.initialized) return;
			c.state.worldId = input.worldId;
			c.state.chunkX = input.chunkX;
			c.state.chunkY = input.chunkY;
			c.state.initialized = true;
		},
		createPlayer: (
			c,
			input: { playerId: string; playerToken: string; playerName: string; x: number; y: number },
		) => {
			c.state.players[input.playerId] = {
				token: input.playerToken,
				connId: null,
				name: input.playerName,
				x: input.x,
				y: input.y,
				inputX: 0,
				inputY: 0,
				sprint: false,
				disconnectedAt: Date.now(),
			};
		},
		setInput: (c, input: { inputX: number; inputY: number; sprint?: boolean }) => {
			const found = findPlayerByConnId(c.state, c.conn.id);
			if (!found) return;
			const [, player] = found;
			player.inputX = Math.max(-1, Math.min(1, input.inputX));
			player.inputY = Math.max(-1, Math.min(1, input.inputY));
			player.sprint = !!input.sprint;
		},
		removePlayer: (c, input: { playerId: string }) => {
			if (isInternalToken(c.conn.params as { internalToken?: string } | undefined)) {
				delete c.state.players[input.playerId];
				broadcastSnapshot(c);
				return;
			}
			const found = findPlayerByConnId(c.state, c.conn.id);
			if (!found) return;
			const [playerId] = found;
			delete c.state.players[playerId];
			broadcastSnapshot(c);
		},
		placeBlock: (c, input: { gridX: number; gridY: number }) => {
			if (!findPlayerByConnId(c.state, c.conn.id)) return;
			if (!c.state.blocks) c.state.blocks = [];
			const { gridX, gridY } = input;
			if (gridX < 0 || gridX >= GRID_COLS || gridY < 0 || gridY >= GRID_COLS) return;
			const key = `${gridX},${gridY}`;
			if (!c.state.blocks.includes(key)) {
				c.state.blocks.push(key);
				broadcastSnapshot(c);
			}
		},
		removeBlock: (c, input: { gridX: number; gridY: number }) => {
			if (!findPlayerByConnId(c.state, c.conn.id)) return;
			if (!c.state.blocks) c.state.blocks = [];
			const key = `${input.gridX},${input.gridY}`;
			const idx = c.state.blocks.indexOf(key);
			if (idx !== -1) {
				c.state.blocks.splice(idx, 1);
				broadcastSnapshot(c);
			}
		},
		getSnapshot: (c) => buildSnapshot(c),
	},
});

interface Snapshot {
	worldId: string;
	chunkX: number;
	chunkY: number;
	chunkSize: number;
	tick: number;
	players: Record<string, { x: number; y: number; name: string }>;
	blocks: string[];
}

function buildSnapshot(c: ActorContextOf<typeof openWorldChunk>): Snapshot {
	const players: Snapshot["players"] = {};
	for (const [id, entry] of Object.entries(c.state.players)) {
		if (entry.disconnectedAt) continue;
		players[id] = { x: entry.x, y: entry.y, name: entry.name };
	}
	return {
		worldId: c.state.worldId,
		chunkX: c.state.chunkX,
		chunkY: c.state.chunkY,
		chunkSize: CHUNK_SIZE,
		tick: c.state.tick,
		players,
		blocks: c.state.blocks,
	};
}

function broadcastSnapshot(c: ActorContextOf<typeof openWorldChunk>) {
	c.broadcast("snapshot", buildSnapshot(c));
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

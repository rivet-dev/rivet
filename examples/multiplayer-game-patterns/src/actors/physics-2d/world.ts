import RAPIER from "@dimforge/rapier2d-compat";
import { type ActorContextOf, actor, event, UserError } from "rivetkit";
import { db } from "rivetkit/db";
import { interval } from "rivetkit/utils";
import { getPlayerColor } from "../player-color.ts";
import {
	JUMP_IMPULSE,
	MOVE_FORCE,
	PLAYER_RADIUS,
	SCENE_DYNAMIC,
	SCENE_STATIC,
	SNAPSHOT_INTERVAL_MS,
	SUB_STEPS,
	TICK_MS,
} from "./config.ts";

const DISCONNECT_GRACE_MS = 5000;

interface PlayerEntry {
	connId: string;
	name: string;
	color: string;
	bodyHandle: number;
	inputX: number;
	jump: boolean;
	disconnectedAt: number | null;
}

type BoxRow = {
	id: string;
	x: number;
	y: number;
	hw: number;
	hh: number;
};

interface BodySnapshot {
	id: string;
	x: number;
	y: number;
	angle: number;
	vx: number;
	vy: number;
	hw: number;
	hh: number;
}

interface Snapshot {
	tick: number;
	serverTime: number;
	bodies: BodySnapshot[];
	players: Record<
		string,
		{ x: number; y: number; name: string; color: string }
	>;
}

export const physics2dWorld = actor({
	options: { name: "Physics 2D - World", icon: "cubes" },
	events: {
		snapshot: event<Snapshot>(),
	},
	// docs:start world
	onBeforeConnect: (_c, params: { name?: string }) => {
		const name = params?.name?.trim();
		if (!name) {
			throw new UserError("name required", { code: "auth_required" });
		}
		if (name.length > 20) {
			throw new UserError("name too long", { code: "invalid_name" });
		}
	},
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS boxes (
					id TEXT PRIMARY KEY,
					x REAL NOT NULL,
					y REAL NOT NULL,
					hw REAL NOT NULL,
					hh REAL NOT NULL
				)
			`);
			await db.execute(`
				CREATE TABLE IF NOT EXISTS world (
					id INTEGER PRIMARY KEY CHECK (id = 1),
					tick INTEGER NOT NULL
				)
			`);
		},
	}),
	createVars: () => ({
		tick: 0,
		world: null as RAPIER.World | null,
		players: {} as Record<string, PlayerEntry>,
		dynamicHandles: {} as Record<string, number>,
		dynamicSizes: {} as Record<string, { hw: number; hh: number }>,
		// `onWake` stores its physics loop here so `onSleep` and `onDestroy` can
		// wait for it.
		loop: null as Promise<void> | null,
	}),
	onConnect: (c, conn) => {
		const name = (conn.params as { name?: string })?.name || "Player";
		const world = getWorld(c);

		const bodyDesc = RAPIER.RigidBodyDesc.dynamic()
			.setTranslation(3 + Math.random() * 6, 5)
			.setCcdEnabled(true);
		const body = world.createRigidBody(bodyDesc);

		const colliderDesc = RAPIER.ColliderDesc.ball(PLAYER_RADIUS)
			.setFriction(0.5)
			.setRestitution(0);
		world.createCollider(colliderDesc, body);

		c.vars.players[conn.id] = {
			connId: conn.id,
			name,
			color: getPlayerColor(conn.id),
			bodyHandle: body.handle,
			inputX: 0,
			jump: false,
			disconnectedAt: null,
		};

		broadcastSnapshot(c);
	},
	onDisconnect: (c, conn) => {
		const player = c.vars.players[conn.id];
		if (!player) return;
		player.disconnectedAt = Date.now();
		broadcastSnapshot(c);
	},
	// docs:end world
	// docs:start wake
	// `onWake` runs on every start. The Actor accepts connections and actions
	// only after it resolves, so the world is loaded before anyone joins.
	onWake: async (c) => {
		const world = await loadWorld(c);
		c.vars.world = world;
		c.vars.loop = runWorld(c, world).catch((error) => {
			c.log.error({ msg: "physics loop failed", error });
		});
	},
	// `c.abortSignal` fires on shutdown, so the loop saves and exits on its own.
	onSleep: async (c) => {
		await c.vars.loop;
	},
	// The database closes right after `onDestroy`, so wait for the final save.
	onDestroy: async (c) => {
		await c.vars.loop;
	},
	// docs:end wake
	actions: {
		// docs:start input
		setInput: (c, input: { inputX: number; jump?: boolean }) => {
			const player = c.vars.players[c.conn.id];
			if (!player) return;
			player.inputX = Math.max(-1, Math.min(1, input.inputX));
			if (input.jump) player.jump = true;
		},
		spawnBox: (c, input: { x: number; y: number }) => {
			const world = getWorld(c);
			const id = `spawned-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`;
			const hw = 0.2 + Math.random() * 0.2;
			const hh = 0.2 + Math.random() * 0.2;
			const angle = Math.random() * Math.PI * 2;

			const bodyDesc = RAPIER.RigidBodyDesc.dynamic()
				.setTranslation(input.x, input.y)
				.setRotation(angle)
				.setLinearDamping(0.5)
				.setCcdEnabled(true);
			const body = world.createRigidBody(bodyDesc);
			const colliderDesc = RAPIER.ColliderDesc.cuboid(hw, hh)
				.setFriction(0.6)
				.setRestitution(0);
			world.createCollider(colliderDesc, body);
			c.vars.dynamicHandles[id] = body.handle;
			c.vars.dynamicSizes[id] = { hw, hh };
			broadcastSnapshot(c);
		},
		// docs:end input
		getSnapshot: (c) => buildSnapshot(c),
	},
});

/**
 * Returns the Rapier world. `onWake` builds it before the Actor accepts
 * connections or actions, so a missing world means startup did not finish.
 */
function getWorld(c: ActorContextOf<typeof physics2dWorld>): RAPIER.World {
	const world = c.vars.world;
	if (!world) {
		throw new Error("physics world is not loaded");
	}
	return world;
}

function buildSnapshot(c: ActorContextOf<typeof physics2dWorld>): Snapshot {
	const world = getWorld(c);
	const bodies: BodySnapshot[] = [];

	// Dynamic scene bodies.
	for (const [id, handle] of Object.entries(c.vars.dynamicHandles)) {
		const body = world.getRigidBody(handle);
		if (!body) continue;
		const pos = body.translation();
		const vel = body.linvel();
		const size = c.vars.dynamicSizes[id] ?? { hw: 0.35, hh: 0.35 };
		bodies.push({
			id,
			x: pos.x,
			y: pos.y,
			angle: body.rotation(),
			vx: vel.x,
			vy: vel.y,
			hw: size.hw,
			hh: size.hh,
		});
	}

	// Player bodies.
	const players: Snapshot["players"] = {};
	for (const [id, entry] of Object.entries(c.vars.players)) {
		if (entry.disconnectedAt) continue;
		const body = world.getRigidBody(entry.bodyHandle);
		if (!body) continue;
		const pos = body.translation();
		const vel = body.linvel();
		bodies.push({
			id: `player-${id}`,
			x: pos.x,
			y: pos.y,
			angle: body.rotation(),
			vx: vel.x,
			vy: vel.y,
			hw: PLAYER_RADIUS,
			hh: PLAYER_RADIUS,
		});
		players[id] = {
			x: pos.x,
			y: pos.y,
			name: entry.name,
			color: entry.color,
		};
	}

	return { tick: c.vars.tick, serverTime: Date.now(), bodies, players };
}

function broadcastSnapshot(c: ActorContextOf<typeof physics2dWorld>) {
	c.broadcast("snapshot", buildSnapshot(c));
}

// docs:start load
/**
 * Builds the Rapier world and restores the tick and boxes from SQLite, or seeds
 * the boxes from config on the first start.
 */
async function loadWorld(
	c: ActorContextOf<typeof physics2dWorld>,
): Promise<RAPIER.World> {
	await RAPIER.init();

	const gravity = new RAPIER.Vector2(0, 20);
	const world = new RAPIER.World(gravity);
	world.timestep = TICK_MS / 1000 / SUB_STEPS;

	// Create static bodies.
	for (const s of SCENE_STATIC) {
		const bodyDesc = RAPIER.RigidBodyDesc.fixed().setTranslation(s.x, s.y);
		const body = world.createRigidBody(bodyDesc);
		const colliderDesc = RAPIER.ColliderDesc.cuboid(s.hw, s.hh)
			.setFriction(0.8)
			.setRestitution(0);
		world.createCollider(colliderDesc, body);
	}

	// Load the last saved world from SQLite, or seed it from config on
	// the first run.
	const [savedWorld] = await c.db.execute<{ tick: number }>(
		"SELECT tick FROM world WHERE id = 1",
	);
	c.vars.tick = savedWorld?.tick ?? 0;
	const savedBoxes = await c.db.execute<BoxRow>(
		"SELECT id, x, y, hw, hh FROM boxes",
	);
	const boxes = savedBoxes.length > 0 ? savedBoxes : SCENE_DYNAMIC;
	for (const { id, x, y, hw, hh } of boxes) {
		const bodyDesc = RAPIER.RigidBodyDesc.dynamic()
			.setTranslation(x, y)
			.setLinearDamping(0.5)
			.setCcdEnabled(true);
		const body = world.createRigidBody(bodyDesc);
		const colliderDesc = RAPIER.ColliderDesc.cuboid(hw, hh)
			.setFriction(0.6)
			.setRestitution(0);
		world.createCollider(colliderDesc, body);
		c.vars.dynamicHandles[id] = body.handle;
		c.vars.dynamicSizes[id] = { hw, hh };
	}

	return world;
}
// docs:end load

// docs:start tick
/**
 * Steps the world every `TICK_MS` until the Actor starts shutting down.
 */
async function runWorld(
	c: ActorContextOf<typeof physics2dWorld>,
	world: RAPIER.World,
): Promise<void> {
	const tick = interval(TICK_MS);
	let savedAt = Date.now();
	while (!c.aborted) {
		await tick();
		if (c.aborted) break;
		c.vars.tick += 1;

		const now = Date.now();

		// Apply player forces and remove disconnected players.
		for (const [id, player] of Object.entries(c.vars.players)) {
			if (
				player.disconnectedAt &&
				now - player.disconnectedAt > DISCONNECT_GRACE_MS
			) {
				const body = world.getRigidBody(player.bodyHandle);
				if (body) world.removeRigidBody(body);
				delete c.vars.players[id];
				continue;
			}

			const body = world.getRigidBody(player.bodyHandle);
			if (!body) continue;

			if (player.inputX !== 0) {
				body.applyImpulse(
					new RAPIER.Vector2(MOVE_FORCE * player.inputX, 0),
					true,
				);
			}

			if (player.jump) {
				player.jump = false;
				const vel = body.linvel();
				if (Math.abs(vel.y) < 2) {
					body.applyImpulse(
						new RAPIER.Vector2(0, -JUMP_IMPULSE),
						true,
					);
				}
			}

			// Damp horizontal velocity only so gravity isn't affected.
			const vel = body.linvel();
			body.setLinvel(new RAPIER.Vector2(vel.x * 0.85, vel.y), true);
		}

		for (let i = 0; i < SUB_STEPS; i++) {
			world.step();
		}

		broadcastSnapshot(c);

		// Ticks only change memory. Save to SQLite every
		// SNAPSHOT_INTERVAL_MS instead of on every tick.
		if (now - savedAt >= SNAPSHOT_INTERVAL_MS) {
			await saveWorld(c, world);
			savedAt = now;
		}
	}

	// Save once more when the Actor stops, so a clean restart loses nothing.
	await saveWorld(c, world);
}
// docs:end tick

// docs:start save
/**
 * Writes the tick and every box position to SQLite in one transaction, so a
 * crash leaves either the previous save or this one, never part of both.
 */
async function saveWorld(
	c: ActorContextOf<typeof physics2dWorld>,
	world: RAPIER.World,
) {
	const tick = c.vars.tick;
	const boxes: BoxRow[] = [];
	for (const [id, handle] of Object.entries(c.vars.dynamicHandles)) {
		const body = world.getRigidBody(handle);
		if (!body) continue;
		const pos = body.translation();
		const size = c.vars.dynamicSizes[id];
		boxes.push({ id, x: pos.x, y: pos.y, hw: size.hw, hh: size.hh });
	}

	await c.db.transaction(
		async (tx) => {
			await tx.execute(
				"INSERT OR REPLACE INTO world (id, tick) VALUES (1, ?)",
				tick,
			);
			await tx.execute("DELETE FROM boxes");
			for (const box of boxes) {
				await tx.execute(
					"INSERT INTO boxes (id, x, y, hw, hh) VALUES (?, ?, ?, ?, ?)",
					box.id,
					box.x,
					box.y,
					box.hw,
					box.hh,
				);
			}
		},
		{ name: "save-world" },
	);
}
// docs:end save

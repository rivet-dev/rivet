import { actor, type ActorContextOf, event, UserError } from "rivetkit";
import { interval } from "rivetkit/utils";
import RAPIER from "@dimforge/rapier2d-compat";
import { getPlayerColor } from "../player-color.ts";
import {
	TICK_MS,
	SUB_STEPS,
	MOVE_FORCE,
	JUMP_IMPULSE,
	PLAYER_RADIUS,
	SCENE_STATIC,
	SCENE_DYNAMIC,
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
	players: Record<string, { x: number; y: number; name: string; color: string }>;
}

export const physics2dWorld = actor({
	options: { name: "Physics 2D - World", icon: "cubes" },
	events: {
		snapshot: event<Snapshot>(),
	},
	onBeforeConnect: (_c, params: { name?: string }) => {
		const name = params?.name?.trim();
		if (!name) {
			throw new UserError("name required", { code: "auth_required" });
		}
		if (name.length > 20) {
			throw new UserError("name too long", { code: "invalid_name" });
		}
	},
	state: {
		tick: 0,
		// [id, x, y, hw, hh] — persisted every tick so positions survive restarts.
		boxes: [] as [string, number, number, number, number][],
	},
	createVars: () => ({
		world: null as RAPIER.World | null,
		players: {} as Record<string, PlayerEntry>,
		dynamicHandles: {} as Record<string, number>,
		dynamicSizes: {} as Record<string, { hw: number; hh: number }>,
	}),
	onConnect: (c, conn) => {
		const name = (conn.params as { name?: string })?.name || "Player";
		const world = c.vars.world;
		if (!world) return;

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
	run: async (c) => {
		await RAPIER.init();

		const gravity = new RAPIER.Vector2(0, 20);
		const world = new RAPIER.World(gravity);
		world.timestep = TICK_MS / 1000 / SUB_STEPS;
		c.vars.world = world;

		// Create static bodies.
		for (const s of SCENE_STATIC) {
			const bodyDesc = RAPIER.RigidBodyDesc.fixed().setTranslation(s.x, s.y);
			const body = world.createRigidBody(bodyDesc);
			const colliderDesc = RAPIER.ColliderDesc.cuboid(s.hw, s.hh)
				.setFriction(0.8)
				.setRestitution(0);
			world.createCollider(colliderDesc, body);
		}

		// Load dynamic bodies from persisted state, or seed from config on first run.
		const boxes = c.state.boxes.length > 0
			? c.state.boxes
			: SCENE_DYNAMIC.map((d): [string, number, number, number, number] => [d.id, d.x, d.y, d.hw, d.hh]);
		for (const [id, x, y, hw, hh] of boxes) {
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

		const tick = interval(TICK_MS);
		while (!c.aborted) {
			await tick();
			if (c.aborted) break;
			c.state.tick += 1;

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

			// Persist current dynamic body positions so they survive restarts.
			c.state.boxes = [];
			for (const [id, handle] of Object.entries(c.vars.dynamicHandles)) {
				const body = world.getRigidBody(handle);
				if (!body) continue;
				const pos = body.translation();
				const size = c.vars.dynamicSizes[id];
				c.state.boxes.push([id, pos.x, pos.y, size.hw, size.hh]);
			}

			broadcastSnapshot(c);
		}
	},
	actions: {
		setInput: (c, input: { inputX: number; jump?: boolean }) => {
			const player = c.vars.players[c.conn.id];
			if (!player) return;
			player.inputX = Math.max(-1, Math.min(1, input.inputX));
			if (input.jump) player.jump = true;
		},
		spawnBox: (c, input: { x: number; y: number }) => {
			const world = c.vars.world;
			if (!world) return;
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
		getSnapshot: (c) => buildSnapshot(c),
	},
});

function buildSnapshot(c: ActorContextOf<typeof physics2dWorld>): Snapshot {
	const world = c.vars.world;
	if (!world) return { tick: c.state.tick, serverTime: Date.now(), bodies: [], players: {} };

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
		players[id] = { x: pos.x, y: pos.y, name: entry.name, color: entry.color };
	}

	return { tick: c.state.tick, serverTime: Date.now(), bodies, players };
}

function broadcastSnapshot(c: ActorContextOf<typeof physics2dWorld>) {
	c.broadcast("snapshot", buildSnapshot(c));
}

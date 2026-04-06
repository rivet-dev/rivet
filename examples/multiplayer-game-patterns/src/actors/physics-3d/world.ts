import { actor, type ActorContextOf, event, UserError } from "rivetkit";
import { interval } from "rivetkit/utils";
import RAPIER from "@dimforge/rapier3d-compat";
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
	inputZ: number;
	jump: boolean;
	disconnectedAt: number | null;
}

interface BodySnapshot {
	id: string;
	x: number;
	y: number;
	z: number;
	hx: number;
	hy: number;
	hz: number;
	qx: number;
	qy: number;
	qz: number;
	qw: number;
	vx: number;
	vy: number;
	vz: number;
}

interface Snapshot {
	tick: number;
	serverTime: number;
	bodies: BodySnapshot[];
	players: Record<string, { x: number; y: number; z: number; name: string; color: string }>;
}

export const physics3dWorld = actor({
	options: { name: "Physics 3D - World", icon: "cube" },
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
		// [id, x, y, z, hx, hy, hz] — persisted every tick so positions survive restarts.
		boxes: [] as [string, number, number, number, number, number, number][],
	},
	createVars: () => ({
		world: null as RAPIER.World | null,
		players: {} as Record<string, PlayerEntry>,
		dynamicHandles: {} as Record<string, number>,
		dynamicSizes: {} as Record<string, { hx: number; hy: number; hz: number }>,
	}),
	onConnect: (c, conn) => {
		const name = (conn.params as { name?: string })?.name || "Player";
		const world = c.vars.world;
		if (!world) return;

		const angle = Math.random() * Math.PI * 2;
		const dist = 2 + Math.random() * 3;
		const bodyDesc = RAPIER.RigidBodyDesc.dynamic()
			.setTranslation(Math.cos(angle) * dist, 2, Math.sin(angle) * dist)
			.setCcdEnabled(true)
			.lockRotations();
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
			inputZ: 0,
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

		const gravity = new RAPIER.Vector3(0, -9.81, 0);
		const world = new RAPIER.World(gravity);
		world.timestep = TICK_MS / 1000 / SUB_STEPS;
		c.vars.world = world;

		// Create static bodies.
		for (const s of SCENE_STATIC) {
			const bodyDesc = RAPIER.RigidBodyDesc.fixed().setTranslation(
				s.x,
				s.y,
				s.z,
			);
			const body = world.createRigidBody(bodyDesc);
			const colliderDesc = RAPIER.ColliderDesc.cuboid(s.hx, s.hy, s.hz)
				.setFriction(0.8)
				.setRestitution(0);
			world.createCollider(colliderDesc, body);
		}

		// Load dynamic bodies from persisted state, or seed from config on first run.
		const boxes = c.state.boxes.length > 0
			? c.state.boxes
			: SCENE_DYNAMIC.map((d): [string, number, number, number, number, number, number] => [d.id, d.x, d.y, d.z, d.hx, d.hy, d.hz]);
		for (const [id, x, y, z, hx, hy, hz] of boxes) {
			const bodyDesc = RAPIER.RigidBodyDesc.dynamic()
				.setTranslation(x, y, z)
				.setLinearDamping(0.5)
				.setCcdEnabled(true);
			const body = world.createRigidBody(bodyDesc);
			const colliderDesc = RAPIER.ColliderDesc.cuboid(hx, hy, hz)
				.setFriction(0.6)
				.setRestitution(0);
			world.createCollider(colliderDesc, body);
			c.vars.dynamicHandles[id] = body.handle;
			c.vars.dynamicSizes[id] = { hx, hy, hz };
		}

		const tick = interval(TICK_MS);
		while (!c.aborted) {
			await tick();
			if (c.aborted) break;
			c.state.tick += 1;

			const now = Date.now();

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

				if (player.inputX !== 0 || player.inputZ !== 0) {
					body.applyImpulse(
						new RAPIER.Vector3(
							MOVE_FORCE * player.inputX,
							0,
							MOVE_FORCE * player.inputZ,
						),
						true,
					);
				}

				if (player.jump) {
					player.jump = false;
					const vel = body.linvel();
					if (Math.abs(vel.y) < 0.5) {
						body.applyImpulse(
							new RAPIER.Vector3(0, JUMP_IMPULSE, 0),
							true,
						);
					}
				}

				// Damp horizontal velocity only so gravity isn't affected.
				const vel = body.linvel();
				body.setLinvel(new RAPIER.Vector3(vel.x * 0.85, vel.y, vel.z * 0.85), true);
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
				c.state.boxes.push([id, pos.x, pos.y, pos.z, size.hx, size.hy, size.hz]);
			}

			broadcastSnapshot(c);
		}
	},
	actions: {
		setInput: (
			c,
			input: { inputX: number; inputZ: number; jump?: boolean },
		) => {
			const player = c.vars.players[c.conn.id];
			if (!player) return;
			player.inputX = Math.max(-1, Math.min(1, input.inputX));
			player.inputZ = Math.max(-1, Math.min(1, input.inputZ));
			if (input.jump) player.jump = true;
		},
		spawnBox: (c, input: { x: number; z: number }) => {
			const world = c.vars.world;
			if (!world) return;
			const id = `spawned-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`;
			const h = 0.2 + Math.random() * 0.3;
			const rotation = randomQuaternion();

			const bodyDesc = RAPIER.RigidBodyDesc.dynamic()
				.setTranslation(input.x, 5, input.z)
				.setRotation(rotation)
				.setLinearDamping(0.5)
				.setCcdEnabled(true);
			const body = world.createRigidBody(bodyDesc);
			const colliderDesc = RAPIER.ColliderDesc.cuboid(h, h, h)
				.setFriction(0.6)
				.setRestitution(0);
			world.createCollider(colliderDesc, body);
			c.vars.dynamicHandles[id] = body.handle;
			c.vars.dynamicSizes[id] = { hx: h, hy: h, hz: h };
			broadcastSnapshot(c);
		},
		getSnapshot: (c) => buildSnapshot(c),
	},
});

function buildSnapshot(c: ActorContextOf<typeof physics3dWorld>): Snapshot {
	const world = c.vars.world;
	if (!world) return { tick: c.state.tick, serverTime: Date.now(), bodies: [], players: {} };

	const bodies: BodySnapshot[] = [];

	for (const [id, handle] of Object.entries(c.vars.dynamicHandles)) {
		const body = world.getRigidBody(handle);
		if (!body) continue;
		const pos = body.translation();
		const rot = body.rotation();
		const vel = body.linvel();
		const size = c.vars.dynamicSizes[id] ?? { hx: 0.25, hy: 0.25, hz: 0.25 };
		bodies.push({
			id,
			x: pos.x,
			y: pos.y,
			z: pos.z,
			hx: size.hx,
			hy: size.hy,
			hz: size.hz,
			qx: rot.x,
			qy: rot.y,
			qz: rot.z,
			qw: rot.w,
			vx: vel.x,
			vy: vel.y,
			vz: vel.z,
		});
	}

	const players: Snapshot["players"] = {};
	for (const [id, entry] of Object.entries(c.vars.players)) {
		if (entry.disconnectedAt) continue;
		const body = world.getRigidBody(entry.bodyHandle);
		if (!body) continue;
		const pos = body.translation();
		const rot = body.rotation();
		const vel = body.linvel();
		bodies.push({
			id: `player-${id}`,
			x: pos.x,
			y: pos.y,
			z: pos.z,
			hx: PLAYER_RADIUS,
			hy: PLAYER_RADIUS,
			hz: PLAYER_RADIUS,
			qx: rot.x,
			qy: rot.y,
			qz: rot.z,
			qw: rot.w,
			vx: vel.x,
			vy: vel.y,
			vz: vel.z,
		});
		players[id] = {
			x: pos.x,
			y: pos.y,
			z: pos.z,
			name: entry.name,
			color: entry.color,
		};
	}

	return { tick: c.state.tick, serverTime: Date.now(), bodies, players };
}

function broadcastSnapshot(c: ActorContextOf<typeof physics3dWorld>) {
	c.broadcast("snapshot", buildSnapshot(c));
}

function randomQuaternion(): { x: number; y: number; z: number; w: number } {
	// Shoemake method for uniform random 3D rotation.
	const u1 = Math.random();
	const u2 = Math.random();
	const u3 = Math.random();
	const sqrt1MinusU1 = Math.sqrt(1 - u1);
	const sqrtU1 = Math.sqrt(u1);
	const theta1 = 2 * Math.PI * u2;
	const theta2 = 2 * Math.PI * u3;
	return {
		x: sqrt1MinusU1 * Math.sin(theta1),
		y: sqrt1MinusU1 * Math.cos(theta1),
		z: sqrtU1 * Math.sin(theta2),
		w: sqrtU1 * Math.cos(theta2),
	};
}

export const TICK_MS = 100; // 10 TPS
export const SUB_STEPS = 4; // Physics sub-steps per tick for stable stacking.
export const MOVE_FORCE = 0.5;
export const JUMP_IMPULSE = 2;
export const PLAYER_RADIUS = 0.4;
export const CORRECTION_ALPHA = 0.3;

// Static bodies.
export const SCENE_STATIC = [
	{ id: "ground", x: 0, y: -0.5, z: 0, hx: 10, hy: 0.5, hz: 10 },
	{ id: "wall-n", x: 0, y: 1, z: -10.25, hx: 10, hy: 1.5, hz: 0.25 },
	{ id: "wall-s", x: 0, y: 1, z: 10.25, hx: 10, hy: 1.5, hz: 0.25 },
	{ id: "wall-w", x: -10.25, y: 1, z: 0, hx: 0.25, hy: 1.5, hz: 10.5 },
	{ id: "wall-e", x: 10.25, y: 1, z: 0, hx: 0.25, hy: 1.5, hz: 10.5 },
];

// Dynamic bodies.
export const SCENE_DYNAMIC = [
	{ id: "cube-0", x: 2, y: 0.5, z: 0, hx: 0.5, hy: 0.5, hz: 0.5 },
	{ id: "cube-1", x: -1, y: 0.5, z: 2, hx: 0.5, hy: 0.5, hz: 0.5 },
	{ id: "cube-2", x: 0, y: 0.5, z: -3, hx: 0.5, hy: 0.5, hz: 0.5 },
	{ id: "cube-3", x: 3, y: 0.5, z: 3, hx: 0.4, hy: 0.4, hz: 0.4 },
	{ id: "cube-4", x: -3, y: 0.5, z: -1, hx: 0.3, hy: 0.6, hz: 0.3 },
	{ id: "cube-5", x: 0, y: 1.5, z: 0, hx: 0.5, hy: 0.5, hz: 0.5 },
];

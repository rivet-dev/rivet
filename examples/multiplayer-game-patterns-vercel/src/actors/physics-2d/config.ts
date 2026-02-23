export const TICK_MS = 100; // 10 TPS
export const SUB_STEPS = 4; // Physics sub-steps per tick for stable stacking.
export const MOVE_FORCE = 1.5;
export const JUMP_IMPULSE = 4;
export const PLAYER_RADIUS = 0.3;
export const CORRECTION_ALPHA = 0.3;

// Scale: 50 pixels per physics unit. Canvas 600x600 = 12x12 units.
export const SCALE = 50;

// Static bodies (immovable).
export const SCENE_STATIC = [
	{ id: "ground", x: 6, y: 11.75, hw: 6, hh: 0.25 },
	{ id: "wall-l", x: -0.25, y: 6, hw: 0.25, hh: 6 },
	{ id: "wall-r", x: 12.25, y: 6, hw: 0.25, hh: 6 },
	{ id: "plat-1", x: 3, y: 8, hw: 1.5, hh: 0.12 },
	{ id: "plat-2", x: 9, y: 6.5, hw: 1.5, hh: 0.12 },
];

// Dynamic bodies (affected by physics).
export const SCENE_DYNAMIC = [
	{ id: "box-0", x: 6, y: 10.5, hw: 0.35, hh: 0.35 },
	{ id: "box-1", x: 6.05, y: 9.8, hw: 0.35, hh: 0.35 },
	{ id: "box-2", x: 5.95, y: 9.1, hw: 0.35, hh: 0.35 },
	{ id: "box-3", x: 2, y: 10.5, hw: 0.3, hh: 0.3 },
	{ id: "box-4", x: 10, y: 10.5, hw: 0.25, hh: 0.4 },
	{ id: "box-5", x: 7.5, y: 10.5, hw: 0.4, hh: 0.25 },
];

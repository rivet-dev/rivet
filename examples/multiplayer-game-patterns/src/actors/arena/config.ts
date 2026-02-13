export type Mode = "1v1" | "ffa" | "duo" | "squad";

export const MODE_CONFIG: Record<Mode, { capacity: number; teams: number }> = {
	"1v1": { capacity: 2, teams: 2 },
	ffa: { capacity: 4, teams: 0 },
	duo: { capacity: 4, teams: 2 },
	squad: { capacity: 8, teams: 2 },
};

export const TICK_MS = 50; // 20 tps
export const WORLD_SIZE = 600;
export const MAX_SPEED = 200; // pixels per second
export const SHOOT_RANGE = 300;
export const SHOOT_ANGLE = 0.15; // radians tolerance (~8.5°)
export const SCORE_LIMIT = 10;

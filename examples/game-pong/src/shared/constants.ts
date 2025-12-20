export const CANVAS_WIDTH = 800;
export const CANVAS_HEIGHT = 400;
export const PADDLE_HEIGHT = 80;
export const PADDLE_WIDTH = 10;
export const BALL_SIZE = 10;
export const PADDLE_SPEED = 8;
export const BALL_SPEED = 5;
export const TICK_RATE = 1000 / 60; // 60 FPS

export type PlayerSide = "left" | "right";

export type GameState = {
	ball: { x: number; y: number; vx: number; vy: number };
	leftPaddle: { y: number };
	rightPaddle: { y: number };
	score: { left: number; right: number };
	leftInput: "up" | "down" | null;
	rightInput: "up" | "down" | null;
	player1: string | null;
	player2: string | null;
	gameStarted: boolean;
};

export type GameStateEvent = {
	ball: { x: number; y: number; vx: number; vy: number };
	leftPaddle: { y: number };
	rightPaddle: { y: number };
	score: { left: number; right: number };
	gameStarted: boolean;
};

export type MatchResult =
	| { matchId: string; status: "matched" }
	| { matchId: null; status: "waiting" };

export type AssignedPlayer = {
	player: PlayerSide;
};

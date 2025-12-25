export const WORLD_WIDTH = 2000;
export const WORLD_HEIGHT = 2000;
// Viewport will be set dynamically based on window size
export const VIEWPORT_WIDTH = 800; // Default, overridden by frontend
export const VIEWPORT_HEIGHT = 600; // Default, overridden by frontend
export const MIN_PLAYER_RADIUS = 20;
export const PLAYER_SPEED = 5;
export const TICK_RATE = 1000 / 60; // 60 FPS

// Generate random colors for players
export const PLAYER_COLORS = [
	"#ff6b6b",
	"#4ecdc4",
	"#45b7d1",
	"#96ceb4",
	"#ffeaa7",
	"#dfe6e9",
	"#fd79a8",
	"#a29bfe",
	"#6c5ce7",
	"#00b894",
];

export type Player = {
	id: string;
	x: number;
	y: number;
	radius: number;
	color: string;
	targetX: number;
	targetY: number;
};

export type Food = {
	id: number;
	x: number;
	y: number;
	color: string;
};

export type GameStateEvent = {
	players: Player[];
	food: Food[];
};

export const FOOD_COUNT = 200;
export const FOOD_RADIUS = 5;
export const FOOD_VALUE = 3; // How much radius increases when eating food
export const MAX_LOBBY_SIZE = 10;

export type PlayerInput = {
	targetX: number;
	targetY: number;
};

export type LobbyInfo = {
	lobbyId: string;
};

export type Position = { x: number; y: number };

export type Player = {
	id: string;
	x: number;
	y: number;
	color: string;
	lastUpdate: number;
};

export type GameState = {
	players: Record<string, Player>;
	region: string;
};

export type ConnectionState = {
	playerId: string | null;
};

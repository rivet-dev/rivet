export type Player = {
	id: string;
	name: string;
	x: number;
	y: number;
	color: string;
	mass: number;
	radius: number;
	lastUpdate: number;
};

export type GameState = {
	roomId: string;
	maxPlayers: number;
	players: Record<string, Player>;
	updatedAt: number;
};

export type RoomStats = {
	roomId: string;
	playerCount: number;
	createdAt: number;
	lastUpdatedAt: number;
	maxPlayers: number;
};

export type JoinResult = {
	playerId: string;
	player: Player;
};

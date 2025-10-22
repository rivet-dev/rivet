export type Position = { x: number; y: number };
export type Input = { x: number; y: number };
export type Player = { id: string; position: Position; input: Input };

export type GameVars = {
	gameLoopInterval?: ReturnType<typeof setInterval>;
};

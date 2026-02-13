import { useEffect, useRef } from "react";
import type { GameClient } from "../../client.ts";
import type { Physics2dMatchInfo } from "./menu.tsx";
import { Physics2dGame } from "./physics-2d-game.ts";

export function Physics2dGameView({
	client,
	matchInfo,
	onLeave,
}: {
	client: GameClient;
	matchInfo: Physics2dMatchInfo;
	onLeave: () => void;
}) {
	const canvasRef = useRef<HTMLCanvasElement>(null);
	const gameRef = useRef<Physics2dGame | null>(null);

	useEffect(() => {
		const canvas = canvasRef.current;
		if (!canvas) return;

		gameRef.current = new Physics2dGame(canvas, client, matchInfo);
		return () => {
			gameRef.current?.destroy();
			gameRef.current = null;
		};
	}, [client, matchInfo]);

	return (
		<div className="app">
			<div className="game-header">
				<h2>Physics 2D</h2>
				<div className="btn-row">
					<button className="btn btn-secondary" onClick={onLeave}>Leave</button>
				</div>
			</div>
			<p className="controls-hint">WASD to move, Space to jump, Click to spawn box. Shared Rapier 2D physics with network smoothing.</p>
			<canvas
				ref={canvasRef}
				width={600}
				height={600}
				className="game-canvas"
			/>
		</div>
	);
}

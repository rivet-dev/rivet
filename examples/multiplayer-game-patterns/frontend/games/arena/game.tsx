import { useEffect, useRef } from "react";
import type { GameClient } from "../../client.ts";
import type { ArenaMatchInfo } from "./menu.tsx";
import { ArenaGame } from "./arena-game.ts";

export function ArenaGameView({
	client,
	matchInfo,
	onLeave,
}: {
	client: GameClient;
	matchInfo: ArenaMatchInfo;
	onLeave: () => void;
}) {
	const canvasRef = useRef<HTMLCanvasElement>(null);
	const gameRef = useRef<ArenaGame | null>(null);

	useEffect(() => {
		const canvas = canvasRef.current;
		if (!canvas) return;

		gameRef.current = new ArenaGame(canvas, client, matchInfo);
		return () => {
			gameRef.current?.destroy();
			gameRef.current = null;
		};
	}, [client, matchInfo]);

	return (
		<div className="app">
			<div className="game-header">
				<h2>Arena ({matchInfo.mode.toUpperCase()})</h2>
				<div className="btn-row">
					<button className="btn btn-secondary" onClick={onLeave}>Leave</button>
				</div>
			</div>
			<p className="controls-hint">WASD to move, click to shoot</p>
			<canvas
				ref={canvasRef}
				width={600}
				height={600}
				className="game-canvas"
			/>
		</div>
	);
}

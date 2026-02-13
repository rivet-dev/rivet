import { useEffect, useRef } from "react";
import type { GameClient } from "../../client.ts";
import type { RankedMatchInfo } from "./menu.tsx";
import { RankedGame } from "./ranked-game.ts";

export function RankedGameView({
	client,
	matchInfo,
	onLeave,
}: {
	client: GameClient;
	matchInfo: RankedMatchInfo;
	onLeave: () => void;
}) {
	const canvasRef = useRef<HTMLCanvasElement>(null);
	const gameRef = useRef<RankedGame | null>(null);

	useEffect(() => {
		const canvas = canvasRef.current;
		if (!canvas) return;

		gameRef.current = new RankedGame(canvas, client, matchInfo);
		return () => {
			gameRef.current?.destroy();
			gameRef.current = null;
		};
	}, [client, matchInfo]);

	return (
		<div className="app">
			<div className="game-header">
				<h2>Ranked ({matchInfo.username} - ELO: {matchInfo.rating})</h2>
				<div className="btn-row">
					<button className="btn btn-secondary" onClick={onLeave}>Leave</button>
				</div>
			</div>
			<p className="controls-hint">WASD to move, click to shoot. First to 5 kills.</p>
			<canvas
				ref={canvasRef}
				width={600}
				height={600}
				className="game-canvas"
			/>
		</div>
	);
}

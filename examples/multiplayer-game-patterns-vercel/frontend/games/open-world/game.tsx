import { useEffect, useRef } from "react";
import type { GameClient } from "../../client.ts";
import type { OpenWorldMatchInfo } from "./menu.tsx";
import { OpenWorldGame } from "./open-world-game.ts";
import { OpenWorldBot } from "./bot.ts";

export function OpenWorldGameView({
	client,
	matchInfo,
	onLeave,
}: {
	client: GameClient;
	matchInfo: OpenWorldMatchInfo;
	onLeave: () => void;
}) {
	const canvasRef = useRef<HTMLCanvasElement>(null);
	const gameRef = useRef<OpenWorldGame | null>(null);
	const botsRef = useRef<OpenWorldBot[]>([]);

	useEffect(() => {
		const canvas = canvasRef.current;
		if (!canvas) return;

		gameRef.current = new OpenWorldGame(canvas, client, matchInfo);
		return () => {
			gameRef.current?.destroy();
			gameRef.current = null;
			for (const bot of botsRef.current) bot.destroy();
			botsRef.current = [];
		};
	}, [client, matchInfo]);

	const addBot = () => {
		botsRef.current.push(new OpenWorldBot(client));
	};

	return (
		<div className="app">
			<div className="game-header">
				<h2>Open World</h2>
				<div className="btn-row">
					<button className="btn btn-secondary" onClick={addBot}>Add Bot</button>
					<button className="btn btn-secondary" onClick={onLeave}>Leave</button>
				</div>
			</div>
			<p className="controls-hint">WASD to move, Shift to sprint, +/- to zoom, LMB to place block, RMB to remove. Walk beyond chunk edges to transfer.</p>
			<canvas
				ref={canvasRef}
				width={600}
				height={600}
				className="game-canvas"
			/>
		</div>
	);
}

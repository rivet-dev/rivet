import { useEffect, useRef } from "react";
import type { GameClient } from "../../client.ts";
import type { IoStyleMatchInfo } from "./menu.tsx";
import { IoGame } from "./io-game.ts";
import { IoBot } from "./bot.ts";

export function IoStyleGame({
	client,
	matchInfo,
	onLeave,
}: {
	client: GameClient;
	matchInfo: IoStyleMatchInfo;
	onLeave: () => void;
}) {
	const canvasRef = useRef<HTMLCanvasElement>(null);
	const gameRef = useRef<IoGame | null>(null);
	const botsRef = useRef<IoBot[]>([]);

	useEffect(() => {
		const canvas = canvasRef.current;
		if (!canvas) return;

		gameRef.current = new IoGame(canvas, client, matchInfo);
		return () => {
			gameRef.current?.destroy();
			gameRef.current = null;
			for (const bot of botsRef.current) bot.destroy();
			botsRef.current = [];
		};
	}, [client, matchInfo]);

	const addBot = () => {
		botsRef.current.push(new IoBot(client));
	};

	return (
		<div className="app">
			<div className="game-header">
				<h2>IO-Style</h2>
				<div className="btn-row">
					<button className="btn btn-secondary" onClick={addBot}>Add Bot</button>
					<button className="btn btn-secondary" onClick={onLeave}>Leave</button>
				</div>
			</div>
			<p className="controls-hint">WASD or arrow keys to move</p>
			<canvas
				ref={canvasRef}
				width={600}
				height={600}
				className="game-canvas"
			/>
		</div>
	);
}

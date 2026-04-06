import { useEffect, useRef } from "react";
import type { GameClient } from "../../client.ts";
import type { BattleRoyaleMatchInfo } from "./menu.tsx";
import { BattleRoyaleGame } from "./battle-royale-game.ts";
import { BattleRoyaleBot } from "./bot.ts";

export function BattleRoyaleGameView({
	client,
	matchInfo,
	onLeave,
}: {
	client: GameClient;
	matchInfo: BattleRoyaleMatchInfo;
	onLeave: () => void;
}) {
	const canvasRef = useRef<HTMLCanvasElement>(null);
	const gameRef = useRef<BattleRoyaleGame | null>(null);
	const botsRef = useRef<BattleRoyaleBot[]>([]);

	useEffect(() => {
		const canvas = canvasRef.current;
		if (!canvas) return;

		gameRef.current = new BattleRoyaleGame(canvas, client, matchInfo);
		return () => {
			gameRef.current?.destroy();
			gameRef.current = null;
			for (const bot of botsRef.current) bot.destroy();
			botsRef.current = [];
		};
	}, [client, matchInfo]);

	const addBot = () => {
		botsRef.current.push(new BattleRoyaleBot(client));
	};

	return (
		<div className="app">
			<div className="game-header">
				<h2>Battle Royale</h2>
				<div className="btn-row">
					<button className="btn btn-secondary" onClick={addBot}>Add Bot</button>
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

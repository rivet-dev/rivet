import { useEffect, useRef } from "react";
import type { GameClient } from "../../client.ts";
import type { Physics3dMatchInfo } from "./menu.tsx";
import { Physics3dGame } from "./physics-3d-game.ts";

export function Physics3dGameView({
	client,
	matchInfo,
	onLeave,
}: {
	client: GameClient;
	matchInfo: Physics3dMatchInfo;
	onLeave: () => void;
}) {
	const containerRef = useRef<HTMLDivElement>(null);
	const gameRef = useRef<Physics3dGame | null>(null);

	useEffect(() => {
		const container = containerRef.current;
		if (!container) return;

		gameRef.current = new Physics3dGame(container, client, matchInfo);
		return () => {
			gameRef.current?.destroy();
			gameRef.current = null;
		};
	}, [client, matchInfo]);

	return (
		<div className="app">
			<div className="game-header">
				<h2>Physics 3D</h2>
				<div className="btn-row">
					<button className="btn btn-secondary" onClick={onLeave}>Leave</button>
				</div>
			</div>
			<p className="controls-hint">WASD to move, Space to jump. Shared Rapier 3D physics with network smoothing.</p>
			<div
				ref={containerRef}
				style={{ width: 600, height: 600, borderRadius: 8, overflow: "hidden" }}
			/>
		</div>
	);
}

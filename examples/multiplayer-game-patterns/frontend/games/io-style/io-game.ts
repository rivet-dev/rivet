import type { GameClient } from "../../client.ts";
import type { IoStyleMatchInfo } from "./menu.tsx";

const PLAYER_RADIUS = 12;
const LERP_FACTOR = 0.2;
const GRID_SPACING = 50;

function colorFromId(id: string): string {
	let hash = 0;
	for (let i = 0; i < id.length; i++) {
		hash = (hash * 31 + id.charCodeAt(i)) | 0;
	}
	const hue = ((hash % 360) + 360) % 360;
	return `hsl(${hue}, 70%, 55%)`;
}

type IoStyleMatchConn = ReturnType<
	ReturnType<GameClient["ioStyleMatch"]["get"]>["connect"]
>;

export class IoGame {
	private stopped = false;
	private rafId = 0;
	private worldSize = 600;
	private targets: Record<string, { x: number; y: number }> = {};
	private display: Record<string, { x: number; y: number }> = {};
	private keys: Record<string, boolean> = {};
	private lastIx = 0;
	private lastIy = 0;
	private botInterval = 0;
	private conn: IoStyleMatchConn;

	constructor(
		private canvas: HTMLCanvasElement | null,
		client: GameClient,
		private matchInfo: IoStyleMatchInfo,
		private options: { bot?: boolean } = {},
	) {
		this.conn = client.ioStyleMatch
			.get([matchInfo.matchId], {
				params: { playerToken: matchInfo.playerToken },
			})
			.connect();

		this.conn.on("snapshot", (raw: unknown) => {
			const snap = raw as {
				worldSize: number;
				players: Record<string, { x: number; y: number }>;
			};
			this.worldSize = snap.worldSize;
			for (const [id, pos] of Object.entries(snap.players)) {
				this.targets[id] = pos;
				if (!this.display[id]) this.display[id] = { x: pos.x, y: pos.y };
			}
			for (const id of Object.keys(this.targets)) {
				if (!snap.players[id]) {
					delete this.targets[id];
					delete this.display[id];
				}
			}
		});

		if (options.bot) {
			this.botInterval = window.setInterval(() => {
				const ix = Math.floor(Math.random() * 3) - 1;
				const iy = Math.floor(Math.random() * 3) - 1;
				this.conn.setInput({ inputX: ix, inputY: iy }).catch(() => {});
			}, 500);
		} else if (canvas) {
			window.addEventListener("keydown", this.onKeyDown);
			window.addEventListener("keyup", this.onKeyUp);
			this.rafId = requestAnimationFrame(this.draw);
		}
	}

	destroy() {
		this.stopped = true;
		cancelAnimationFrame(this.rafId);
		clearInterval(this.botInterval);
		if (!this.options.bot) {
			window.removeEventListener("keydown", this.onKeyDown);
			window.removeEventListener("keyup", this.onKeyUp);
		}
		this.conn.dispose().catch(() => {});
	}

	private onKeyDown = (e: KeyboardEvent) => {
		this.keys[e.key] = true;
		this.sendInput();
	};

	private onKeyUp = (e: KeyboardEvent) => {
		this.keys[e.key] = false;
		this.sendInput();
	};

	private sendInput() {
		let ix = 0;
		let iy = 0;
		if (this.keys["w"] || this.keys["W"] || this.keys["ArrowUp"]) iy -= 1;
		if (this.keys["s"] || this.keys["S"] || this.keys["ArrowDown"]) iy += 1;
		if (this.keys["a"] || this.keys["A"] || this.keys["ArrowLeft"]) ix -= 1;
		if (this.keys["d"] || this.keys["D"] || this.keys["ArrowRight"]) ix += 1;
		if (ix !== this.lastIx || iy !== this.lastIy) {
			this.lastIx = ix;
			this.lastIy = iy;
			this.conn.setInput({ inputX: ix, inputY: iy }).catch(() => {});
		}
	}

	private draw = () => {
		if (this.stopped) return;
		const canvas = this.canvas!;
		const ctx = canvas.getContext("2d")!;
		const sx = canvas.width / this.worldSize;
		const sy = canvas.height / this.worldSize;

		ctx.fillStyle = "#000000";
		ctx.fillRect(0, 0, canvas.width, canvas.height);

		ctx.strokeStyle = "#2c2c2e";
		ctx.lineWidth = 1;
		ctx.beginPath();
		for (let x = 0; x <= this.worldSize; x += GRID_SPACING) {
			const px = x * sx;
			ctx.moveTo(px, 0);
			ctx.lineTo(px, canvas.height);
		}
		for (let y = 0; y <= this.worldSize; y += GRID_SPACING) {
			const py = y * sy;
			ctx.moveTo(0, py);
			ctx.lineTo(canvas.width, py);
		}
		ctx.stroke();

		for (const [id, target] of Object.entries(this.targets)) {
			const d = this.display[id];
			if (!d) continue;
			d.x += (target.x - d.x) * LERP_FACTOR;
			d.y += (target.y - d.y) * LERP_FACTOR;

			const px = d.x * sx;
			const py = d.y * sy;
			const isMe = id === this.matchInfo.playerId;

			ctx.beginPath();
			ctx.arc(px, py, PLAYER_RADIUS, 0, Math.PI * 2);
			ctx.fillStyle = isMe ? "#ff4f00" : colorFromId(id);
			ctx.fill();
			if (isMe) {
				ctx.lineWidth = 2;
				ctx.strokeStyle = "#ffffff";
				ctx.stroke();
			}

			ctx.fillStyle = "#ffffff";
			ctx.font = "11px sans-serif";
			ctx.textAlign = "center";
			ctx.fillText(id.slice(0, 6), px, py - PLAYER_RADIUS - 4);
		}

		this.rafId = requestAnimationFrame(this.draw);
	};
}

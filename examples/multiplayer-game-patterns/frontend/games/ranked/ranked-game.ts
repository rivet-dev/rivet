import type { GameClient } from "../../client.ts";
import type { RankedMatchInfo } from "./menu.tsx";

const PLAYER_RADIUS = 12;
const LERP_FACTOR = 0.2;
const GRID_SPACING = 50;
const SEND_RATE_MS = 50;
const MOVE_SPEED = 200;
const SHOT_LINE_DURATION = 150;
const RUBBER_BAND_THRESHOLD = 40;

function colorFromId(id: string): string {
	let hash = 0;
	for (let i = 0; i < id.length; i++) {
		hash = (hash * 31 + id.charCodeAt(i)) | 0;
	}
	const hue = ((hash % 360) + 360) % 360;
	return `hsl(${hue}, 70%, 55%)`;
}

interface ShotLine {
	fromX: number;
	fromY: number;
	toX: number;
	toY: number;
	hit: boolean;
	createdAt: number;
}

export class RankedGame {
	private stopped = false;
	private rafId = 0;
	private worldSize = 600;
	private targets: Record<string, { x: number; y: number; score: number; rating: number }> = {};
	private display: Record<string, { x: number; y: number }> = {};
	private keys: Record<string, boolean> = {};
	private phase: "waiting" | "live" | "finished" = "waiting";
	private winnerId: string | null = null;
	private scoreLimit = 5;
	private shotLines: ShotLine[] = [];
	private lastSendTime = 0;
	private localX = 300;
	private localY = 300;
	private lastFrameTime = 0;
	private botInterval = 0;
	private conn: {
		updatePosition: (i: { x: number; y: number }) => Promise<unknown>;
		shoot: (i: { dirX: number; dirY: number }) => Promise<unknown>;
		on: (e: string, cb: (d: unknown) => void) => void;
		dispose: () => Promise<void>;
	};

	constructor(
		private canvas: HTMLCanvasElement | null,
		client: GameClient,
		private matchInfo: RankedMatchInfo,
		private options: { bot?: boolean } = {},
	) {
		this.conn = client.rankedMatch
			.get([matchInfo.matchId], {
				params: { username: matchInfo.username },
			})
			.connect() as typeof this.conn;

		this.conn.on("snapshot", (raw: unknown) => {
			const snap = raw as {
				worldSize: number;
				phase: "waiting" | "live" | "finished";
				winnerId: string | null;
				scoreLimit: number;
				players: Record<string, { x: number; y: number; score: number; rating: number }>;
			};
			this.worldSize = snap.worldSize;
			this.phase = snap.phase;
			this.winnerId = snap.winnerId;
			this.scoreLimit = snap.scoreLimit;

			for (const [id, data] of Object.entries(snap.players)) {
				this.targets[id] = data;
				if (id === this.matchInfo.username) {
					const dx = data.x - this.localX;
					const dy = data.y - this.localY;
					if (Math.sqrt(dx * dx + dy * dy) > RUBBER_BAND_THRESHOLD) {
						this.localX = data.x;
						this.localY = data.y;
					}
				} else if (!this.display[id]) {
					this.display[id] = { x: data.x, y: data.y };
				}
			}
			for (const id of Object.keys(this.targets)) {
				if (!snap.players[id]) {
					delete this.targets[id];
					delete this.display[id];
				}
			}
		});

		this.conn.on("shoot", (raw: unknown) => {
			const shot = raw as {
				fromX: number;
				fromY: number;
				dirX: number;
				dirY: number;
				hitPlayerId: string | null;
			};
			this.shotLines.push({
				fromX: shot.fromX,
				fromY: shot.fromY,
				toX: shot.fromX + shot.dirX * 300,
				toY: shot.fromY + shot.dirY * 300,
				hit: shot.hitPlayerId !== null,
				createdAt: performance.now(),
			});
		});

		if (options.bot) {
			this.botInterval = window.setInterval(() => {
				if (this.phase !== "live") return;
				this.localX += (Math.random() - 0.5) * 40;
				this.localY += (Math.random() - 0.5) * 40;
				this.localX = Math.max(0, Math.min(this.worldSize, this.localX));
				this.localY = Math.max(0, Math.min(this.worldSize, this.localY));
				this.conn.updatePosition({ x: this.localX, y: this.localY }).catch(() => {});
				if (Math.random() < 0.2) {
					const angle = Math.random() * Math.PI * 2;
					this.conn.shoot({ dirX: Math.cos(angle), dirY: Math.sin(angle) }).catch(() => {});
				}
			}, 100);
		} else if (canvas) {
			canvas.addEventListener("click", this.onClick);
			window.addEventListener("keydown", this.onKeyDown);
			window.addEventListener("keyup", this.onKeyUp);
			this.lastFrameTime = performance.now();
			this.rafId = requestAnimationFrame(this.draw);
		}
	}

	destroy() {
		this.stopped = true;
		cancelAnimationFrame(this.rafId);
		clearInterval(this.botInterval);
		if (!this.options.bot && this.canvas) {
			this.canvas.removeEventListener("click", this.onClick);
			window.removeEventListener("keydown", this.onKeyDown);
			window.removeEventListener("keyup", this.onKeyUp);
		}
		this.conn.dispose().catch(() => {});
	}

	private onKeyDown = (e: KeyboardEvent) => { this.keys[e.key] = true; };
	private onKeyUp = (e: KeyboardEvent) => { this.keys[e.key] = false; };

	private onClick = (e: MouseEvent) => {
		if (this.phase !== "live" || !this.canvas) return;
		const rect = this.canvas.getBoundingClientRect();
		const sx = this.canvas.width / this.worldSize;
		const sy = this.canvas.height / this.worldSize;
		const clickX = (e.clientX - rect.left) / sx;
		const clickY = (e.clientY - rect.top) / sy;
		const dx = clickX - this.localX;
		const dy = clickY - this.localY;
		const mag = Math.sqrt(dx * dx + dy * dy);
		if (mag === 0) return;
		this.conn.shoot({ dirX: dx / mag, dirY: dy / mag }).catch(() => {});
	};

	private draw = () => {
		if (this.stopped) return;
		const now = performance.now();
		const dt = (now - this.lastFrameTime) / 1000;
		this.lastFrameTime = now;
		const canvas = this.canvas!;
		const ctx = canvas.getContext("2d")!;
		const sx = canvas.width / this.worldSize;
		const sy = canvas.height / this.worldSize;

		if (this.phase === "live") {
			let ix = 0;
			let iy = 0;
			if (this.keys["w"] || this.keys["W"] || this.keys["ArrowUp"]) iy -= 1;
			if (this.keys["s"] || this.keys["S"] || this.keys["ArrowDown"]) iy += 1;
			if (this.keys["a"] || this.keys["A"] || this.keys["ArrowLeft"]) ix -= 1;
			if (this.keys["d"] || this.keys["D"] || this.keys["ArrowRight"]) ix += 1;
			if (ix !== 0 || iy !== 0) {
				const mag = Math.sqrt(ix * ix + iy * iy);
				this.localX = Math.max(0, Math.min(this.worldSize, this.localX + (ix / mag) * MOVE_SPEED * dt));
				this.localY = Math.max(0, Math.min(this.worldSize, this.localY + (iy / mag) * MOVE_SPEED * dt));
			}
			if (now - this.lastSendTime >= SEND_RATE_MS) {
				this.lastSendTime = now;
				this.conn.updatePosition({ x: this.localX, y: this.localY }).catch(() => {});
			}
		}

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

		this.shotLines = this.shotLines.filter((s) => now - s.createdAt < SHOT_LINE_DURATION);
		for (const shot of this.shotLines) {
			const alpha = 1 - (now - shot.createdAt) / SHOT_LINE_DURATION;
			ctx.strokeStyle = shot.hit
				? `rgba(255, 59, 48, ${alpha})`
				: `rgba(255, 255, 255, ${alpha * 0.4})`;
			ctx.lineWidth = 2;
			ctx.beginPath();
			ctx.moveTo(shot.fromX * sx, shot.fromY * sy);
			ctx.lineTo(shot.toX * sx, shot.toY * sy);
			ctx.stroke();
		}

		for (const [id, target] of Object.entries(this.targets)) {
			const isMe = id === this.matchInfo.username;
			let px: number;
			let py: number;

			if (isMe) {
				px = this.localX * sx;
				py = this.localY * sy;
			} else {
				const d = this.display[id];
				if (!d) continue;
				d.x += (target.x - d.x) * LERP_FACTOR;
				d.y += (target.y - d.y) * LERP_FACTOR;
				px = d.x * sx;
				py = d.y * sy;
			}

			const color = isMe ? "#ff4f00" : colorFromId(id);

			ctx.beginPath();
			ctx.arc(px, py, PLAYER_RADIUS, 0, Math.PI * 2);
			ctx.fillStyle = color;
			ctx.fill();
			if (isMe) {
				ctx.lineWidth = 2;
				ctx.strokeStyle = "#ffffff";
				ctx.stroke();
			}

			ctx.fillStyle = "#ffffff";
			ctx.font = "11px sans-serif";
			ctx.textAlign = "center";
			ctx.fillText(
				`${id} [${target.score}/${this.scoreLimit}] R:${target.rating}`,
				px,
				py - PLAYER_RADIUS - 4,
			);
		}

		// Score overlay.
		ctx.fillStyle = "rgba(0, 0, 0, 0.6)";
		ctx.fillRect(0, 0, canvas.width, 28);
		ctx.fillStyle = "#ffffff";
		ctx.font = "12px sans-serif";
		ctx.textAlign = "left";
		const myData = this.targets[this.matchInfo.username];
		const scoreText = myData ? `Score: ${myData.score}/${this.scoreLimit}  Rating: ${myData.rating}` : "";
		ctx.fillText(`${this.phase.toUpperCase()}  ${scoreText}`, 8, 18);

		if (this.phase === "waiting") {
			ctx.fillStyle = "rgba(0, 0, 0, 0.5)";
			ctx.fillRect(0, 0, canvas.width, canvas.height);
			ctx.fillStyle = "#ffffff";
			ctx.font = "24px sans-serif";
			ctx.textAlign = "center";
			ctx.fillText("Waiting for opponent...", canvas.width / 2, canvas.height / 2);
		} else if (this.phase === "finished") {
			ctx.fillStyle = "rgba(0, 0, 0, 0.5)";
			ctx.fillRect(0, 0, canvas.width, canvas.height);
			ctx.fillStyle = "#ffffff";
			ctx.font = "24px sans-serif";
			ctx.textAlign = "center";
			const winText = this.winnerId === this.matchInfo.username
				? "You Win!"
				: "You Lose!";
			ctx.fillText(winText, canvas.width / 2, canvas.height / 2);
		}

		this.rafId = requestAnimationFrame(this.draw);
	};
}

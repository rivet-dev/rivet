import type { GameClient } from "../../client.ts";
import type { BattleRoyaleMatchInfo } from "./menu.tsx";

const PLAYER_RADIUS = 10;
const LERP_FACTOR = 0.2;
const GRID_SPACING = 50;
const SEND_RATE_MS = 50;
const MOVE_SPEED = 150;
const SHOT_LINE_DURATION = 150;
const RUBBER_BAND_THRESHOLD = 40;
const VIEWPORT_SIZE = 600;

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

export class BattleRoyaleGame {
	private stopped = false;
	private rafId = 0;
	private worldSize = 1200;
	private targets: Record<string, { x: number; y: number; hp: number; maxHp: number; alive: boolean; placement: number | null }> = {};
	private display: Record<string, { x: number; y: number }> = {};
	private keys: Record<string, boolean> = {};
	private phase: "lobby" | "live" | "finished" = "lobby";
	private winnerId: string | null = null;
	private playerCount = 0;
	private aliveCount = 0;
	private capacity = 16;
	private lobbyCountdown: number | null = null;
	private zone = { centerX: 600, centerY: 600, radius: 550 };
	private shotLines: ShotLine[] = [];
	private lastSendTime = 0;
	private localX = 0;
	private localY = 0;
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
		private matchInfo: BattleRoyaleMatchInfo,
		private options: { bot?: boolean } = {},
	) {
		this.conn = client.battleRoyaleMatch
			.get([matchInfo.matchId], {
				params: { playerToken: matchInfo.playerToken },
			})
			.connect() as typeof this.conn;

		this.conn.on("snapshot", (raw: unknown) => {
			const snap = raw as {
				worldSize: number;
				phase: "lobby" | "live" | "finished";
				winnerId: string | null;
				playerCount: number;
				aliveCount: number;
				capacity: number;
				lobbyCountdown: number | null;
				zone: { centerX: number; centerY: number; radius: number };
				players: Record<string, { x: number; y: number; hp: number; maxHp: number; alive: boolean; placement: number | null }>;
			};
			this.worldSize = snap.worldSize;
			this.phase = snap.phase;
			this.winnerId = snap.winnerId;
			this.playerCount = snap.playerCount;
			this.aliveCount = snap.aliveCount;
			this.capacity = snap.capacity;
			this.lobbyCountdown = snap.lobbyCountdown;
			this.zone = snap.zone;

			for (const [id, data] of Object.entries(snap.players)) {
				this.targets[id] = data;
				if (id === this.matchInfo.playerId) {
					// Snap to server position on first snapshot or large rubber-band.
					if (this.localX === 0 && this.localY === 0) {
						this.localX = data.x;
						this.localY = data.y;
					} else {
						const dx = data.x - this.localX;
						const dy = data.y - this.localY;
						if (Math.sqrt(dx * dx + dy * dy) > RUBBER_BAND_THRESHOLD) {
							this.localX = data.x;
							this.localY = data.y;
						}
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
				toX: shot.fromX + shot.dirX * 250,
				toY: shot.fromY + shot.dirY * 250,
				hit: shot.hitPlayerId !== null,
				createdAt: performance.now(),
			});
		});

		if (options.bot) {
			this.botInterval = window.setInterval(() => {
				this.localX += (Math.random() - 0.5) * 40;
				this.localY += (Math.random() - 0.5) * 40;
				this.localX = Math.max(0, Math.min(this.worldSize, this.localX));
				this.localY = Math.max(0, Math.min(this.worldSize, this.localY));
				this.conn.updatePosition({ x: this.localX, y: this.localY }).catch(() => {});
				if (this.phase === "live" && Math.random() < 0.15) {
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
		const myData = this.targets[this.matchInfo.playerId];
		if (!myData?.alive) return;
		const rect = this.canvas.getBoundingClientRect();
		// Click relative to viewport center (player position).
		const clickX = (e.clientX - rect.left) - VIEWPORT_SIZE / 2;
		const clickY = (e.clientY - rect.top) - VIEWPORT_SIZE / 2;
		const mag = Math.sqrt(clickX * clickX + clickY * clickY);
		if (mag === 0) return;
		this.conn.shoot({ dirX: clickX / mag, dirY: clickY / mag }).catch(() => {});
	};

	private draw = () => {
		if (this.stopped) return;
		const now = performance.now();
		const dt = (now - this.lastFrameTime) / 1000;
		this.lastFrameTime = now;
		const canvas = this.canvas!;
		const ctx = canvas.getContext("2d")!;

		const myData = this.targets[this.matchInfo.playerId];
		const myAlive = myData?.alive ?? true;

		// Movement.
		if ((this.phase === "live" || this.phase === "lobby") && myAlive) {
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

		// Camera.
		const camX = this.localX - VIEWPORT_SIZE / 2;
		const camY = this.localY - VIEWPORT_SIZE / 2;

		ctx.fillStyle = "#0a0a0a";
		ctx.fillRect(0, 0, canvas.width, canvas.height);

		// Grid.
		ctx.strokeStyle = "#1a1a1a";
		ctx.lineWidth = 1;
		ctx.beginPath();
		const startX = Math.floor(camX / GRID_SPACING) * GRID_SPACING;
		const startY = Math.floor(camY / GRID_SPACING) * GRID_SPACING;
		for (let x = startX; x <= camX + VIEWPORT_SIZE; x += GRID_SPACING) {
			ctx.moveTo(x - camX, 0);
			ctx.lineTo(x - camX, VIEWPORT_SIZE);
		}
		for (let y = startY; y <= camY + VIEWPORT_SIZE; y += GRID_SPACING) {
			ctx.moveTo(0, y - camY);
			ctx.lineTo(VIEWPORT_SIZE, y - camY);
		}
		ctx.stroke();

		// World boundary.
		ctx.strokeStyle = "#ff3b30";
		ctx.lineWidth = 2;
		ctx.strokeRect(-camX, -camY, this.worldSize, this.worldSize);

		// Zone circle.
		if (this.phase === "live") {
			ctx.beginPath();
			ctx.arc(
				this.zone.centerX - camX,
				this.zone.centerY - camY,
				this.zone.radius,
				0,
				Math.PI * 2,
			);
			ctx.strokeStyle = "rgba(59, 130, 246, 0.6)";
			ctx.lineWidth = 3;
			ctx.stroke();

			// Zone fill outside.
			ctx.save();
			ctx.beginPath();
			ctx.rect(-camX - 100, -camY - 100, this.worldSize + 200, this.worldSize + 200);
			ctx.arc(
				this.zone.centerX - camX,
				this.zone.centerY - camY,
				this.zone.radius,
				0,
				Math.PI * 2,
				true,
			);
			ctx.fillStyle = "rgba(59, 130, 246, 0.08)";
			ctx.fill();
			ctx.restore();
		}

		// Shot lines.
		this.shotLines = this.shotLines.filter((s) => now - s.createdAt < SHOT_LINE_DURATION);
		for (const shot of this.shotLines) {
			const alpha = 1 - (now - shot.createdAt) / SHOT_LINE_DURATION;
			ctx.strokeStyle = shot.hit
				? `rgba(255, 59, 48, ${alpha})`
				: `rgba(255, 255, 255, ${alpha * 0.4})`;
			ctx.lineWidth = 2;
			ctx.beginPath();
			ctx.moveTo(shot.fromX - camX, shot.fromY - camY);
			ctx.lineTo(shot.toX - camX, shot.toY - camY);
			ctx.stroke();
		}

		// Players.
		for (const [id, target] of Object.entries(this.targets)) {
			if (!target.alive && id !== this.matchInfo.playerId) continue;
			const isMe = id === this.matchInfo.playerId;
			let px: number;
			let py: number;

			if (isMe) {
				px = this.localX - camX;
				py = this.localY - camY;
			} else {
				const d = this.display[id];
				if (!d) continue;
				d.x += (target.x - d.x) * LERP_FACTOR;
				d.y += (target.y - d.y) * LERP_FACTOR;
				px = d.x - camX;
				py = d.y - camY;
			}

			if (px < -50 || px > VIEWPORT_SIZE + 50 || py < -50 || py > VIEWPORT_SIZE + 50) continue;

			const color = isMe ? "#ff4f00" : colorFromId(id);

			// Player circle.
			ctx.beginPath();
			ctx.arc(px, py, PLAYER_RADIUS, 0, Math.PI * 2);
			ctx.fillStyle = target.alive ? color : "#555";
			ctx.fill();
			if (isMe) {
				ctx.lineWidth = 2;
				ctx.strokeStyle = "#ffffff";
				ctx.stroke();
			}

			// HP bar.
			if (target.alive && this.phase === "live") {
				const barWidth = 24;
				const barHeight = 3;
				const barX = px - barWidth / 2;
				const barY = py - PLAYER_RADIUS - 8;
				const hpRatio = target.hp / target.maxHp;
				ctx.fillStyle = "#333";
				ctx.fillRect(barX, barY, barWidth, barHeight);
				ctx.fillStyle = hpRatio > 0.5 ? "#30d158" : hpRatio > 0.25 ? "#ff4f00" : "#ff3b30";
				ctx.fillRect(barX, barY, barWidth * hpRatio, barHeight);
			}

			// Label.
			ctx.fillStyle = "#ffffff";
			ctx.font = "10px sans-serif";
			ctx.textAlign = "center";
			ctx.fillText(id.slice(0, 6), px, py - PLAYER_RADIUS - 12);
		}

		// HUD.
		ctx.fillStyle = "rgba(0, 0, 0, 0.6)";
		ctx.fillRect(0, 0, canvas.width, 28);
		ctx.fillStyle = "#ffffff";
		ctx.font = "12px sans-serif";
		ctx.textAlign = "left";
		if (this.phase === "lobby") {
			const countdownText = this.lobbyCountdown !== null
				? `  Starting in ${Math.ceil(this.lobbyCountdown / 10)}...`
				: "";
			ctx.fillText(`LOBBY  ${this.playerCount}/${this.capacity} players${countdownText}`, 8, 18);
		} else if (this.phase === "live") {
			ctx.fillText(`LIVE  ${this.aliveCount} alive`, 8, 18);
		} else {
			ctx.fillText("FINISHED", 8, 18);
		}

		// Phase overlays.
		if (this.phase === "finished") {
			ctx.fillStyle = "rgba(0, 0, 0, 0.5)";
			ctx.fillRect(0, 0, canvas.width, canvas.height);
			ctx.fillStyle = "#ffffff";
			ctx.font = "24px sans-serif";
			ctx.textAlign = "center";
			if (this.winnerId === this.matchInfo.playerId) {
				ctx.fillStyle = "#ffd700";
				ctx.fillText("Victory Royale!", canvas.width / 2, canvas.height / 2);
			} else {
				const placement = myData?.placement;
				ctx.fillText(
					placement ? `Eliminated #${placement}` : "Game Over",
					canvas.width / 2,
					canvas.height / 2,
				);
			}
		}

		this.rafId = requestAnimationFrame(this.draw);
	};
}

import type { GameClient } from "../../client.ts";
import type { Physics2dMatchInfo } from "./menu.tsx";
import {
	SCALE,
	CORRECTION_ALPHA,
	PLAYER_RADIUS,
	SCENE_STATIC,
} from "../../../src/actors/physics-2d/config.ts";

interface BodySnapshot {
	id: string;
	x: number;
	y: number;
	angle: number;
	vx: number;
	vy: number;
	hw: number;
	hh: number;
}

interface Snapshot {
	tick: number;
	serverTime: number;
	bodies: BodySnapshot[];
	players: Record<string, { x: number; y: number; name: string }>;
}

interface DisplayBody {
	x: number;
	y: number;
	angle: number;
}

function colorFromId(id: string): string {
	let hash = 0;
	for (let i = 0; i < id.length; i++) {
		hash = (hash * 31 + id.charCodeAt(i)) | 0;
	}
	const hue = ((hash % 360) + 360) % 360;
	return `hsl(${hue}, 70%, 55%)`;
}

export class Physics2dGame {
	private stopped = false;
	private rafId = 0;
	private keys: Record<string, boolean> = {};
	private lastIx = 0;
	private lastJump = false;
	private targets: Record<string, BodySnapshot> = {};
	private display: Record<string, DisplayBody> = {};
	private playerNames: Record<string, string> = {};
	private myConnId = "";
	private lastSnapshotTime = 0;
	private tickIntervalMs = 0;
	private latencyMs = 0;
	private conn: {
		setInput: (i: { inputX: number; jump?: boolean }) => Promise<unknown>;
		spawnBox: (i: { x: number; y: number }) => Promise<unknown>;
		on: (e: string, cb: (d: unknown) => void) => void;
		dispose: () => Promise<void>;
	};

	constructor(
		private canvas: HTMLCanvasElement,
		client: GameClient,
		matchInfo: Physics2dMatchInfo,
	) {
		const handle = client.physics2dWorld.getOrCreate(["main"], {
			params: { name: matchInfo.name },
		});
		this.conn = handle.connect() as typeof this.conn;

		// Capture connection id from the handle.
		(handle as unknown as { id: Promise<string> }).id?.then?.((id: string) => {
			this.myConnId = id;
		});

		this.conn.on("snapshot", (raw: unknown) => {
			const snap = raw as Snapshot;
			const now = Date.now();

			// Measure tick interval and one-way latency estimate.
			if (this.lastSnapshotTime > 0) {
				this.tickIntervalMs = now - this.lastSnapshotTime;
			}
			this.lastSnapshotTime = now;
			this.latencyMs = Math.max(0, now - snap.serverTime);

			// Track player names and find our connection ID from players map.
			for (const [id, info] of Object.entries(snap.players)) {
				this.playerNames[id] = info.name;
				if (info.name === matchInfo.name && !this.myConnId) {
					this.myConnId = id;
				}
			}

			for (const body of snap.bodies) {
				this.targets[body.id] = body;
				if (!this.display[body.id]) {
					this.display[body.id] = {
						x: body.x,
						y: body.y,
						angle: body.angle,
					};
				}
			}

			// Remove stale bodies.
			const currentIds = new Set(snap.bodies.map((b) => b.id));
			for (const id of Object.keys(this.targets)) {
				if (!currentIds.has(id)) {
					delete this.targets[id];
					delete this.display[id];
				}
			}

			// Remove stale players.
			for (const id of Object.keys(this.playerNames)) {
				if (!snap.players[id]) {
					delete this.playerNames[id];
				}
			}
		});

		this.canvas.addEventListener("mousedown", this.onMouseDown);
		window.addEventListener("keydown", this.onKeyDown);
		window.addEventListener("keyup", this.onKeyUp);
		this.rafId = requestAnimationFrame(this.draw);
	}

	destroy() {
		this.stopped = true;
		cancelAnimationFrame(this.rafId);
		this.canvas.removeEventListener("mousedown", this.onMouseDown);
		window.removeEventListener("keydown", this.onKeyDown);
		window.removeEventListener("keyup", this.onKeyUp);
		this.conn.dispose().catch(() => {});
	}

	private onMouseDown = (e: MouseEvent) => {
		const rect = this.canvas.getBoundingClientRect();
		const x = (e.clientX - rect.left) / SCALE;
		const y = (e.clientY - rect.top) / SCALE;
		this.conn.spawnBox({ x, y }).catch(() => {});
	};

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
		if (this.keys["a"] || this.keys["A"] || this.keys["ArrowLeft"]) ix -= 1;
		if (this.keys["d"] || this.keys["D"] || this.keys["ArrowRight"]) ix += 1;
		const jump = !!this.keys[" "];

		if (ix !== this.lastIx || (jump && !this.lastJump)) {
			this.lastIx = ix;
			this.lastJump = jump;
			this.conn.setInput({ inputX: ix, jump }).catch(() => {});
		}
		if (!jump && this.lastJump) {
			this.lastJump = false;
		}
	}

	private draw = () => {
		if (this.stopped) return;
		const ctx = this.canvas.getContext("2d")!;
		const W = this.canvas.width;
		const H = this.canvas.height;

		ctx.fillStyle = "#000000";
		ctx.fillRect(0, 0, W, H);

		// Lerp display positions toward server targets.
		for (const [id, target] of Object.entries(this.targets)) {
			const d = this.display[id]!;
			d.x += (target.x - d.x) * CORRECTION_ALPHA;
			d.y += (target.y - d.y) * CORRECTION_ALPHA;

			// Shortest-path angle interpolation to handle wrapping around ±π.
			let angleDiff = target.angle - d.angle;
			angleDiff = ((angleDiff + Math.PI) % (2 * Math.PI)) - Math.PI;
			if (angleDiff < -Math.PI) angleDiff += 2 * Math.PI;
			d.angle += angleDiff * CORRECTION_ALPHA;
		}

		// Draw static bodies.
		ctx.fillStyle = "#3a3a3c";
		for (const s of SCENE_STATIC) {
			ctx.fillRect(
				(s.x - s.hw) * SCALE,
				(s.y - s.hh) * SCALE,
				s.hw * 2 * SCALE,
				s.hh * 2 * SCALE,
			);
		}

		// Draw dynamic bodies (from snapshot, includes spawned boxes).
		for (const [id, target] of Object.entries(this.targets)) {
			if (id.startsWith("player-")) continue;
			const disp = this.display[id];
			if (!disp) continue;

			ctx.save();
			ctx.translate(disp.x * SCALE, disp.y * SCALE);
			ctx.rotate(disp.angle);
			ctx.fillStyle = colorFromId(id);
			ctx.fillRect(
				-target.hw * SCALE,
				-target.hh * SCALE,
				target.hw * 2 * SCALE,
				target.hh * 2 * SCALE,
			);
			ctx.restore();
		}

		// Draw player circles.
		for (const [connId, name] of Object.entries(this.playerNames)) {
			const bodyId = `player-${connId}`;
			const disp = this.display[bodyId];
			if (!disp) continue;

			const px = disp.x * SCALE;
			const py = disp.y * SCALE;
			const isMe = connId === this.myConnId;

			ctx.beginPath();
			ctx.arc(px, py, PLAYER_RADIUS * SCALE, 0, Math.PI * 2);
			ctx.fillStyle = isMe ? "#ff4f00" : colorFromId(connId);
			ctx.fill();
			if (isMe) {
				ctx.lineWidth = 2;
				ctx.strokeStyle = "#ffffff";
				ctx.stroke();
			}

			ctx.fillStyle = "#ffffff";
			ctx.font = "11px sans-serif";
			ctx.textAlign = "center";
			ctx.fillText(name, px, py - PLAYER_RADIUS * SCALE - 4);
		}

		// HUD: tick rate and latency.
		ctx.fillStyle = "rgba(0, 0, 0, 0.6)";
		ctx.fillRect(W - 150, 4, 146, 40);
		ctx.fillStyle = "#ffffff";
		ctx.font = "11px monospace";
		ctx.textAlign = "right";
		const tps = this.tickIntervalMs > 0 ? (1000 / this.tickIntervalMs).toFixed(1) : "—";
		ctx.fillText(`TPS: ${tps}  Interval: ${this.tickIntervalMs}ms`, W - 8, 18);
		ctx.fillText(`Latency: ~${this.latencyMs}ms`, W - 8, 34);

		this.rafId = requestAnimationFrame(this.draw);
	};
}

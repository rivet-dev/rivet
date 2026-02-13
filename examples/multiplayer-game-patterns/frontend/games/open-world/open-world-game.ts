import type { GameClient } from "../../client.ts";
import type { OpenWorldMatchInfo } from "./menu.tsx";

const PLAYER_RADIUS = 12;
const LERP_FACTOR = 0.2;
const GRID_SPACING = 50;
const BLOCK_SIZE = 50;
const DEFAULT_VIEWPORT_SIZE = 600;
const MIN_ZOOM = 0.25;
const MAX_ZOOM = 2;
const ZOOM_STEP = 0.15;

function colorFromId(id: string): string {
	let hash = 0;
	for (let i = 0; i < id.length; i++) {
		hash = (hash * 31 + id.charCodeAt(i)) | 0;
	}
	const hue = ((hash % 360) + 360) % 360;
	return `hsl(${hue}, 70%, 55%)`;
}

interface ChunkConnection {
	cx: number;
	cy: number;
	conn: {
		setInput: (i: { inputX: number; inputY: number; sprint?: boolean }) => Promise<unknown>;
		removePlayer: (i: { playerId: string }) => Promise<unknown>;
		placeBlock: (i: { gridX: number; gridY: number }) => Promise<unknown>;
		removeBlock: (i: { gridX: number; gridY: number }) => Promise<unknown>;
		on: (e: string, cb: (d: unknown) => void) => void;
		dispose: () => Promise<void>;
	};
	players: Record<string, { x: number; y: number; name: string }>;
	display: Record<string, { x: number; y: number }>;
	blocks: Set<string>;
}

export class OpenWorldGame {
	private stopped = false;
	private rafId = 0;
	private chunkSize = 1200;
	private chunkX: number;
	private chunkY: number;
	private playerId: string;
	private playerToken: string;
	private keys: Record<string, boolean> = {};
	private lastIx = 0;
	private lastIy = 0;
	private lastSprint = false;
	private transferring = false;
	private mouseCanvasX = -1;
	private mouseCanvasY = -1;
	private lastChunkUpdate = 0;
	private botInterval = 0;
	private zoom = 1;
	private chunks = new Map<string, ChunkConnection>();

	constructor(
		private canvas: HTMLCanvasElement | null,
		private client: GameClient,
		matchInfo: OpenWorldMatchInfo,
		private options: { bot?: boolean } = {},
	) {
		this.chunkX = matchInfo.chunkKey[1];
		this.chunkY = matchInfo.chunkKey[2];
		this.playerId = matchInfo.playerId;
		this.playerToken = matchInfo.playerToken;

		this.connectToChunk(matchInfo.chunkKey[1], matchInfo.chunkKey[2], matchInfo.playerToken, true);
		this.updateAdjacentChunks();

		if (options.bot) {
			this.botInterval = window.setInterval(() => {
				const ix = Math.floor(Math.random() * 3) - 1;
				const iy = Math.floor(Math.random() * 3) - 1;
				this.lastIx = ix;
				this.lastIy = iy;
				const primary = this.chunks.get(chunkKey(this.chunkX, this.chunkY));
				primary?.conn.setInput({ inputX: ix, inputY: iy }).catch(() => {});
			}, 500);
		} else if (canvas) {
			window.addEventListener("keydown", this.onKeyDown);
			window.addEventListener("keyup", this.onKeyUp);
			canvas.addEventListener("mousedown", this.onMouseDown);
			canvas.addEventListener("mousemove", this.onMouseMove);
			canvas.addEventListener("mouseleave", this.onMouseLeave);
			canvas.addEventListener("contextmenu", this.onContextMenu);
			this.rafId = requestAnimationFrame(this.draw);
		}
	}

	private connectToChunk(cx: number, cy: number, playerToken: string | null, isPrimary: boolean) {
		const key = chunkKey(cx, cy);
		if (this.chunks.has(key)) return;

		const params = playerToken
			? { playerToken }
			: { observer: "true" };

		const conn = this.client.openWorldChunk
			.getOrCreate(["default", String(cx), String(cy)], { params })
			.connect() as ChunkConnection["conn"];

		const chunk: ChunkConnection = {
			cx,
			cy,
			conn,
			players: {},
			display: {},
			blocks: new Set(),
		};

		conn.on("snapshot", (raw: unknown) => {
			const snap = raw as {
				chunkSize: number;
				chunkX: number;
				chunkY: number;
				players: Record<string, { x: number; y: number; name: string }>;
				blocks?: string[];
			};
			this.chunkSize = snap.chunkSize;
			for (const [id, pos] of Object.entries(snap.players)) {
				chunk.players[id] = pos;
				if (!chunk.display[id]) chunk.display[id] = { x: pos.x, y: pos.y };
			}
			for (const id of Object.keys(chunk.players)) {
				if (!snap.players[id]) {
					delete chunk.players[id];
					delete chunk.display[id];
				}
			}
			if (snap.blocks) {
				chunk.blocks = new Set(snap.blocks);
			}

			// Client-driven chunk transfer detection (only for primary chunk).
			if (isPrimary) {
				const me = snap.players[this.playerId];
				if (me && !this.transferring) {
					const atLeft = me.x <= 0 && this.lastIx < 0;
					const atRight = me.x >= this.chunkSize - 1 && this.lastIx > 0;
					const atTop = me.y <= 0 && this.lastIy < 0;
					const atBottom = me.y >= this.chunkSize - 1 && this.lastIy > 0;
					if (atLeft || atRight || atTop || atBottom) {
						this.initiateTransfer(me.x, me.y);
					}
				}
			}
		});

		this.chunks.set(key, chunk);
	}

	private disconnectChunk(cx: number, cy: number) {
		const key = chunkKey(cx, cy);
		const chunk = this.chunks.get(key);
		if (chunk) {
			chunk.conn.dispose().catch(() => {});
			this.chunks.delete(key);
		}
	}

	/** Compute which chunk indices are visible based on player position and zoom. */
	private getVisibleChunkRange(): { minCx: number; maxCx: number; minCy: number; maxCy: number } {
		const primaryChunk = this.chunks.get(chunkKey(this.chunkX, this.chunkY));
		const me = primaryChunk?.display[this.playerId];
		const localX = me?.x ?? this.chunkSize / 2;
		const localY = me?.y ?? this.chunkSize / 2;
		const worldX = this.chunkX * this.chunkSize + localX;
		const worldY = this.chunkY * this.chunkSize + localY;

		const viewportSize = DEFAULT_VIEWPORT_SIZE / this.zoom;
		const halfView = viewportSize / 2;

		return {
			minCx: Math.floor((worldX - halfView) / this.chunkSize),
			maxCx: Math.floor((worldX + halfView) / this.chunkSize),
			minCy: Math.floor((worldY - halfView) / this.chunkSize),
			maxCy: Math.floor((worldY + halfView) / this.chunkSize),
		};
	}

	private updateAdjacentChunks() {
		const { minCx, maxCx, minCy, maxCy } = this.getVisibleChunkRange();

		const needed = new Set<string>();
		for (let cx = minCx; cx <= maxCx; cx++) {
			for (let cy = minCy; cy <= maxCy; cy++) {
				needed.add(chunkKey(cx, cy));
			}
		}

		// Disconnect chunks that are no longer needed.
		for (const [key, chunk] of this.chunks) {
			if (!needed.has(key) && !(chunk.cx === this.chunkX && chunk.cy === this.chunkY)) {
				chunk.conn.dispose().catch(() => {});
				this.chunks.delete(key);
			}
		}

		// Connect to new chunks as observer (read-only).
		for (const key of needed) {
			if (!this.chunks.has(key)) {
				const [cx, cy] = parseChunkKey(key);
				this.connectToChunk(cx, cy, null, false);
			}
		}
	}

	private async initiateTransfer(localX: number, localY: number) {
		this.transferring = true;
		try {
			let absX = this.chunkX * this.chunkSize + localX;
			let absY = this.chunkY * this.chunkSize + localY;
			if (this.lastIx < 0) absX -= 1;
			else if (this.lastIx > 0) absX += 1;
			if (this.lastIy < 0) absY -= 1;
			else if (this.lastIy > 0) absY += 1;

			const primaryChunk = this.chunks.get(chunkKey(this.chunkX, this.chunkY));
			const myName = primaryChunk?.players[this.playerId]?.name ?? "Player";

			const index = this.client.openWorldIndex.getOrCreate(["main"]).connect();
			const result = await index.send(
				"getChunkForPosition",
				{ x: absX, y: absY, playerName: myName },
				{ wait: true, timeout: 10_000 },
			);
			index.dispose();
			const response = (
				result as { response?: { chunkKey: [string, number, number]; playerId: string; playerToken: string } }
			)?.response;
			if (!response || this.stopped) return;

			// Remove old player from old chunk so it doesn't appear as a ghost.
			const oldPrimary = this.chunks.get(chunkKey(this.chunkX, this.chunkY));
			const oldPlayerId = this.playerId;
			oldPrimary?.conn.removePlayer({ playerId: oldPlayerId }).catch(() => {});

			// Update identity.
			this.playerId = response.playerId;
			this.playerToken = response.playerToken;
			this.chunkX = response.chunkKey[1];
			this.chunkY = response.chunkKey[2];

			// Disconnect any existing observer connection to the target chunk
			// so connectToChunk can establish a proper primary connection.
			this.disconnectChunk(this.chunkX, this.chunkY);

			// Connect to new primary chunk.
			this.connectToChunk(this.chunkX, this.chunkY, response.playerToken, true);

			// Pre-initialize display position so the player doesn't snap to center.
			const newPrimary = this.chunks.get(chunkKey(this.chunkX, this.chunkY));
			if (newPrimary) {
				const expectedLocalX = absX - this.chunkX * this.chunkSize;
				const expectedLocalY = absY - this.chunkY * this.chunkSize;
				newPrimary.display[response.playerId] = { x: expectedLocalX, y: expectedLocalY };
			}

			// Update visible chunks.
			this.updateAdjacentChunks();

			// Re-send current input to new primary.
			newPrimary?.conn.setInput({ inputX: this.lastIx, inputY: this.lastIy, sprint: this.lastSprint }).catch(() => {});
		} catch {
			// Transfer failed, stay in current chunk.
		} finally {
			this.transferring = false;
		}
	}

	destroy() {
		this.stopped = true;
		cancelAnimationFrame(this.rafId);
		clearInterval(this.botInterval);
		if (!this.options.bot && this.canvas) {
			window.removeEventListener("keydown", this.onKeyDown);
			window.removeEventListener("keyup", this.onKeyUp);
			this.canvas.removeEventListener("mousedown", this.onMouseDown);
			this.canvas.removeEventListener("mousemove", this.onMouseMove);
			this.canvas.removeEventListener("mouseleave", this.onMouseLeave);
			this.canvas.removeEventListener("contextmenu", this.onContextMenu);
		}
		for (const chunk of this.chunks.values()) {
			chunk.conn.dispose().catch(() => {});
		}
		this.chunks.clear();
	}

	private setKey(key: string, value: boolean) {
		this.keys[key] = value;
		// Normalize single characters to lowercase so Shift doesn't leave stuck keys.
		if (key.length === 1) this.keys[key.toLowerCase()] = value;
	}

	private onKeyDown = (e: KeyboardEvent) => {
		// Zoom controls.
		if (e.key === "=" || e.key === "+") {
			this.zoom = Math.min(MAX_ZOOM, this.zoom + ZOOM_STEP);
			this.updateAdjacentChunks();
			return;
		}
		if (e.key === "-" || e.key === "_") {
			this.zoom = Math.max(MIN_ZOOM, this.zoom - ZOOM_STEP);
			this.updateAdjacentChunks();
			return;
		}
		this.setKey(e.key, true);
		this.sendInput();
	};

	private onKeyUp = (e: KeyboardEvent) => {
		this.setKey(e.key, false);
		this.sendInput();
	};

	private onContextMenu = (e: MouseEvent) => {
		e.preventDefault();
	};

	private onMouseMove = (e: MouseEvent) => {
		if (!this.canvas) return;
		const rect = this.canvas.getBoundingClientRect();
		this.mouseCanvasX = e.clientX - rect.left;
		this.mouseCanvasY = e.clientY - rect.top;
	};

	private onMouseLeave = () => {
		this.mouseCanvasX = -1;
		this.mouseCanvasY = -1;
	};

	private onMouseDown = (e: MouseEvent) => {
		if (!this.canvas) return;
		const rect = this.canvas.getBoundingClientRect();
		const canvasX = e.clientX - rect.left;
		const canvasY = e.clientY - rect.top;

		// Convert canvas coords to world coords.
		const viewportSize = DEFAULT_VIEWPORT_SIZE / this.zoom;
		const scale = DEFAULT_VIEWPORT_SIZE / viewportSize;
		const primaryChunk = this.chunks.get(chunkKey(this.chunkX, this.chunkY));
		const meDisplay = primaryChunk?.display[this.playerId];
		const localPlayerX = meDisplay?.x ?? this.chunkSize / 2;
		const localPlayerY = meDisplay?.y ?? this.chunkSize / 2;
		const worldPlayerX = this.chunkX * this.chunkSize + localPlayerX;
		const worldPlayerY = this.chunkY * this.chunkSize + localPlayerY;
		const camX = worldPlayerX - viewportSize / 2;
		const camY = worldPlayerY - viewportSize / 2;

		const worldX = camX + canvasX / scale;
		const worldY = camY + canvasY / scale;

		// Determine which chunk this falls in.
		const targetCx = Math.floor(worldX / this.chunkSize);
		const targetCy = Math.floor(worldY / this.chunkSize);
		const localBlockX = worldX - targetCx * this.chunkSize;
		const localBlockY = worldY - targetCy * this.chunkSize;
		const gridX = Math.floor(localBlockX / BLOCK_SIZE);
		const gridY = Math.floor(localBlockY / BLOCK_SIZE);

		const chunk = this.chunks.get(chunkKey(targetCx, targetCy));
		if (!chunk) return;

		if (e.button === 0) {
			// Left click: place block.
			chunk.conn.placeBlock({ gridX, gridY }).catch(() => {});
		} else if (e.button === 2) {
			// Right click: remove block.
			chunk.conn.removeBlock({ gridX, gridY }).catch(() => {});
		}
	};

	private sendInput() {
		if (this.transferring) return;
		let ix = 0;
		let iy = 0;
		if (this.keys["w"] || this.keys["ArrowUp"]) iy -= 1;
		if (this.keys["s"] || this.keys["ArrowDown"]) iy += 1;
		if (this.keys["a"] || this.keys["ArrowLeft"]) ix -= 1;
		if (this.keys["d"] || this.keys["ArrowRight"]) ix += 1;
		const sprint = !!(this.keys["Shift"]);
		if (ix !== this.lastIx || iy !== this.lastIy || sprint !== this.lastSprint) {
			this.lastIx = ix;
			this.lastIy = iy;
			this.lastSprint = sprint;
			const primary = this.chunks.get(chunkKey(this.chunkX, this.chunkY));
			primary?.conn.setInput({ inputX: ix, inputY: iy, sprint }).catch(() => {});
		}
	}

	private draw = () => {
		if (this.stopped) return;
		const canvas = this.canvas!;
		const ctx = canvas.getContext("2d")!;
		const viewportSize = DEFAULT_VIEWPORT_SIZE / this.zoom;

		// Get player position from primary chunk.
		const primaryChunk = this.chunks.get(chunkKey(this.chunkX, this.chunkY));
		const me = primaryChunk?.players[this.playerId];
		const meDisplay = primaryChunk?.display[this.playerId];
		if (meDisplay && me) {
			meDisplay.x += (me.x - meDisplay.x) * LERP_FACTOR;
			meDisplay.y += (me.y - meDisplay.y) * LERP_FACTOR;
		}

		const localPlayerX = meDisplay?.x ?? this.chunkSize / 2;
		const localPlayerY = meDisplay?.y ?? this.chunkSize / 2;
		const worldPlayerX = this.chunkX * this.chunkSize + localPlayerX;
		const worldPlayerY = this.chunkY * this.chunkSize + localPlayerY;

		// Periodically update which chunks we're connected to as the player moves.
		const now = Date.now();
		if (now - this.lastChunkUpdate > 500) {
			this.lastChunkUpdate = now;
			this.updateAdjacentChunks();
		}

		// Camera centered on player.
		const camX = worldPlayerX - viewportSize / 2;
		const camY = worldPlayerY - viewportSize / 2;
		const scale = DEFAULT_VIEWPORT_SIZE / viewportSize;

		ctx.fillStyle = "#000000";
		ctx.fillRect(0, 0, canvas.width, canvas.height);

		ctx.save();
		ctx.scale(scale, scale);

		// Grid lines in world space.
		ctx.strokeStyle = "#1a1a1a";
		ctx.lineWidth = 1 / scale;
		ctx.beginPath();
		const gridStartX = Math.floor(camX / GRID_SPACING) * GRID_SPACING;
		const gridStartY = Math.floor(camY / GRID_SPACING) * GRID_SPACING;
		for (let x = gridStartX; x <= camX + viewportSize; x += GRID_SPACING) {
			ctx.moveTo(x - camX, 0);
			ctx.lineTo(x - camX, viewportSize);
		}
		for (let y = gridStartY; y <= camY + viewportSize; y += GRID_SPACING) {
			ctx.moveTo(0, y - camY);
			ctx.lineTo(viewportSize, y - camY);
		}
		ctx.stroke();

		// Draw blocks from all connected chunks.
		for (const chunk of this.chunks.values()) {
			for (const blockKey of chunk.blocks) {
				const [gx, gy] = blockKey.split(",").map(Number);
				const wx = chunk.cx * this.chunkSize + gx! * BLOCK_SIZE;
				const wy = chunk.cy * this.chunkSize + gy! * BLOCK_SIZE;
				const sx = wx - camX;
				const sy = wy - camY;
				if (sx + BLOCK_SIZE < 0 || sx > viewportSize || sy + BLOCK_SIZE < 0 || sy > viewportSize) continue;
				ctx.fillStyle = "rgba(255, 79, 0, 0.3)";
				ctx.fillRect(sx, sy, BLOCK_SIZE, BLOCK_SIZE);
				ctx.strokeStyle = "rgba(255, 79, 0, 0.6)";
				ctx.lineWidth = 1 / scale;
				ctx.strokeRect(sx, sy, BLOCK_SIZE, BLOCK_SIZE);
			}
		}

		// Cursor ghost block indicator.
		if (this.mouseCanvasX >= 0 && this.mouseCanvasY >= 0) {
			const mWorldX = camX + this.mouseCanvasX / scale;
			const mWorldY = camY + this.mouseCanvasY / scale;
			const gx = Math.floor(mWorldX / BLOCK_SIZE) * BLOCK_SIZE;
			const gy = Math.floor(mWorldY / BLOCK_SIZE) * BLOCK_SIZE;
			const sx = gx - camX;
			const sy = gy - camY;
			ctx.strokeStyle = "rgba(255, 79, 0, 0.3)";
			ctx.lineWidth = 1 / scale;
			ctx.strokeRect(sx, sy, BLOCK_SIZE, BLOCK_SIZE);
		}

		// Chunk boundaries in world space.
		ctx.strokeStyle = "#2c2c2e";
		ctx.lineWidth = 2 / scale;
		const chunkStartX = Math.floor(camX / this.chunkSize) * this.chunkSize;
		const chunkStartY = Math.floor(camY / this.chunkSize) * this.chunkSize;
		ctx.beginPath();
		for (let x = chunkStartX; x <= camX + viewportSize + this.chunkSize; x += this.chunkSize) {
			ctx.moveTo(x - camX, 0);
			ctx.lineTo(x - camX, viewportSize);
		}
		for (let y = chunkStartY; y <= camY + viewportSize + this.chunkSize; y += this.chunkSize) {
			ctx.moveTo(0, y - camY);
			ctx.lineTo(viewportSize, y - camY);
		}
		ctx.stroke();

		// Chunk index labels at all four corners of each visible chunk.
		ctx.font = `${11 / scale}px ui-monospace, SFMono-Regular, 'SF Mono', Consolas, monospace`;
		ctx.fillStyle = "rgba(255, 255, 255, 0.25)";
		const cxStart = Math.floor(camX / this.chunkSize);
		const cyStart = Math.floor(camY / this.chunkSize);
		const cxEnd = Math.floor((camX + viewportSize) / this.chunkSize);
		const cyEnd = Math.floor((camY + viewportSize) / this.chunkSize);
		const pad = 6 / scale;
		const fontSize = 11 / scale;
		for (let cx = cxStart; cx <= cxEnd; cx++) {
			for (let cy = cyStart; cy <= cyEnd; cy++) {
				const label = `${cx},${cy}`;
				const left = cx * this.chunkSize - camX;
				const top = cy * this.chunkSize - camY;
				const right = left + this.chunkSize;
				const bottom = top + this.chunkSize;

				// Top-left.
				ctx.textAlign = "left";
				ctx.fillText(label, left + pad, top + fontSize + pad);
				// Top-right.
				ctx.textAlign = "right";
				ctx.fillText(label, right - pad, top + fontSize + pad);
				// Bottom-left.
				ctx.textAlign = "left";
				ctx.fillText(label, left + pad, bottom - pad);
				// Bottom-right.
				ctx.textAlign = "right";
				ctx.fillText(label, right - pad, bottom - pad);
			}
		}

		// Draw players from all connected chunks.
		for (const chunk of this.chunks.values()) {
			for (const [id, target] of Object.entries(chunk.players)) {
				if (id === this.playerId && chunk.cx === this.chunkX && chunk.cy === this.chunkY) continue;
				const d = chunk.display[id];
				if (!d) continue;
				d.x += (target.x - d.x) * LERP_FACTOR;
				d.y += (target.y - d.y) * LERP_FACTOR;

				const wx = chunk.cx * this.chunkSize + d.x;
				const wy = chunk.cy * this.chunkSize + d.y;
				const px = wx - camX;
				const py = wy - camY;

				if (px < -50 / scale || px > viewportSize + 50 / scale || py < -50 / scale || py > viewportSize + 50 / scale) continue;

				ctx.beginPath();
				ctx.arc(px, py, PLAYER_RADIUS / scale, 0, Math.PI * 2);
				ctx.fillStyle = colorFromId(id);
				ctx.fill();

				ctx.fillStyle = "#ffffff";
				ctx.font = `${11 / scale}px sans-serif`;
				ctx.textAlign = "center";
				ctx.fillText(target.name || id.slice(0, 6), px, py - PLAYER_RADIUS / scale - 4 / scale);
			}
		}

		// Draw self at center.
		const selfScreenX = viewportSize / 2;
		const selfScreenY = viewportSize / 2;
		ctx.beginPath();
		ctx.arc(selfScreenX, selfScreenY, PLAYER_RADIUS / scale, 0, Math.PI * 2);
		ctx.fillStyle = "#ff4f00";
		ctx.fill();
		ctx.lineWidth = 2 / scale;
		ctx.strokeStyle = "#ffffff";
		ctx.stroke();

		if (me) {
			ctx.fillStyle = "#ffffff";
			ctx.font = `${11 / scale}px sans-serif`;
			ctx.textAlign = "center";
			ctx.fillText(me.name || this.playerId.slice(0, 6), selfScreenX, selfScreenY - PLAYER_RADIUS / scale - 4 / scale);
		}

		ctx.restore();

		// HUD (not affected by zoom).
		ctx.fillStyle = "rgba(0, 0, 0, 0.6)";
		ctx.fillRect(0, 0, canvas.width, 28);
		ctx.fillStyle = "#ffffff";
		ctx.font = "12px ui-monospace, SFMono-Regular, 'SF Mono', Consolas, monospace";
		ctx.textAlign = "left";
		const sprintLabel = this.lastSprint ? "  [SPRINT]" : "";
		ctx.fillText(
			`Chunk: ${this.chunkX},${this.chunkY}  Pos: ${Math.round(localPlayerX)},${Math.round(localPlayerY)}  Zoom: ${this.zoom.toFixed(2)}x${sprintLabel}`,
			8,
			18,
		);

		// Debug: connected chunks list on the right side.
		const chunkEntries = Array.from(this.chunks.entries()).sort(([a], [b]) => a.localeCompare(b));
		const totalPlayers = chunkEntries.reduce((sum, [, c]) => sum + Object.keys(c.players).length, 0);
		ctx.fillStyle = "rgba(0, 0, 0, 0.6)";
		const debugX = canvas.width - 130;
		const lineH = 14;
		const debugH = (chunkEntries.length + 2) * lineH + 8;
		ctx.fillRect(debugX - 6, 0, 136, debugH);

		ctx.fillStyle = "#ffffff";
		ctx.font = "11px ui-monospace, SFMono-Regular, 'SF Mono', Consolas, monospace";
		ctx.textAlign = "left";
		let dy = 14;
		ctx.fillText(`Chunks (${totalPlayers} players):`, debugX, dy);
		dy += lineH;
		for (const [key, chunk] of chunkEntries) {
			const isPrimary = chunk.cx === this.chunkX && chunk.cy === this.chunkY;
			const pCount = Object.keys(chunk.players).length;
			ctx.fillStyle = isPrimary ? "#ff4f00" : "#8e8e93";
			ctx.fillText(`${key}${pCount > 0 ? ` (${pCount})` : ""}${isPrimary ? " *" : ""}`, debugX, dy);
			dy += lineH;
		}
		this.rafId = requestAnimationFrame(this.draw);
	};
}

function chunkKey(cx: number, cy: number): string {
	return `${cx},${cy}`;
}

function parseChunkKey(key: string): [number, number] {
	const [cx, cy] = key.split(",").map(Number);
	return [cx!, cy!];
}

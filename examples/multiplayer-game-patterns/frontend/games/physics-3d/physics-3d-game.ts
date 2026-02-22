import * as THREE from "three";
import type { GameClient } from "../../client.ts";
import type { Physics3dMatchInfo } from "./menu.tsx";
import {
	CORRECTION_ALPHA,
	PLAYER_RADIUS,
	SCENE_STATIC,
} from "../../../src/actors/physics-3d/config.ts";

interface BodySnapshot {
	id: string;
	x: number;
	y: number;
	z: number;
	qx: number;
	qy: number;
	qz: number;
	qw: number;
}

interface Snapshot {
	tick: number;
	serverTime: number;
	bodies: BodySnapshot[];
	players: Record<string, { x: number; y: number; z: number; name: string }>;
}

function colorFromId(id: string): number {
	let hash = 0;
	for (let i = 0; i < id.length; i++) {
		hash = (hash * 31 + id.charCodeAt(i)) | 0;
	}
	const hue = ((hash % 360) + 360) % 360;
	const color = new THREE.Color();
	color.setHSL(hue / 360, 0.7, 0.55);
	return color.getHex();
}

export class Physics3dGame {
	private stopped = false;
	private rafId = 0;
	private keys: Record<string, boolean> = {};
	private lastIx = 0;
	private lastIz = 0;
	private lastJump = false;
	private myConnId = "";
	private playerNames: Record<string, string> = {};
	private lastSnapshotTime = 0;
	private tickIntervalMs = 0;
	private latencyMs = 0;
	private hudEl: HTMLDivElement;

	private scene: THREE.Scene;
	private camera: THREE.PerspectiveCamera;
	private renderer: THREE.WebGLRenderer;

	private dynamicMeshes: Record<string, THREE.Mesh> = {};
	private playerMeshes: Record<string, THREE.Mesh> = {};
	private playerLabels: Record<string, THREE.Sprite> = {};

	private targets: Record<string, BodySnapshot> = {};

	private raycaster = new THREE.Raycaster();
	private groundPlane = new THREE.Plane(new THREE.Vector3(0, 1, 0), 0);

	private conn: {
		setInput: (i: { inputX: number; inputZ: number; jump?: boolean }) => Promise<unknown>;
		spawnBox: (i: { x: number; z: number }) => Promise<unknown>;
		on: (e: string, cb: (d: unknown) => void) => void;
		dispose: () => Promise<void>;
	};

	constructor(
		private container: HTMLDivElement,
		client: GameClient,
		private matchInfo: Physics3dMatchInfo,
	) {
		// Three.js setup.
		this.scene = new THREE.Scene();
		this.scene.background = new THREE.Color(0x111111);
		this.scene.fog = new THREE.Fog(0x111111, 30, 60);

		this.camera = new THREE.PerspectiveCamera(60, 1, 0.1, 100);
		this.camera.position.set(0, 12, 15);
		this.camera.lookAt(0, 0, 0);

		this.renderer = new THREE.WebGLRenderer({ antialias: true });
		this.renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
		this.renderer.setSize(600, 600);
		this.renderer.shadowMap.enabled = true;
		this.renderer.domElement.style.display = "block";
		container.style.position = "relative";
		container.appendChild(this.renderer.domElement);

		// HUD overlay.
		this.hudEl = document.createElement("div");
		this.hudEl.style.cssText = "position:absolute;top:4px;right:8px;color:#fff;font:11px monospace;background:rgba(0,0,0,0.6);padding:4px 8px;border-radius:4px;pointer-events:none;text-align:right;";
		container.appendChild(this.hudEl);

		// Lighting.
		const ambient = new THREE.AmbientLight(0xffffff, 0.4);
		this.scene.add(ambient);

		const dir = new THREE.DirectionalLight(0xffffff, 0.8);
		dir.position.set(5, 10, 5);
		dir.castShadow = true;
		dir.shadow.mapSize.set(1024, 1024);
		dir.shadow.camera.near = 0.5;
		dir.shadow.camera.far = 50;
		dir.shadow.camera.left = -20;
		dir.shadow.camera.right = 20;
		dir.shadow.camera.top = 20;
		dir.shadow.camera.bottom = -20;
		this.scene.add(dir);

		// Grid helper.
		const grid = new THREE.GridHelper(40, 40, 0x333333, 0x222222);
		grid.position.y = 0.01;
		this.scene.add(grid);

		// Static bodies (ground).
		for (const s of SCENE_STATIC) {
			const geo = new THREE.BoxGeometry(s.hx * 2, s.hy * 2, s.hz * 2);
			const mat = new THREE.MeshStandardMaterial({ color: 0x3a3a3c });
			const mesh = new THREE.Mesh(geo, mat);
			mesh.position.set(s.x, s.y, s.z);
			mesh.receiveShadow = true;
			this.scene.add(mesh);
		}

		// Connect to actor.
		const handle = client.physics3dWorld.getOrCreate(["main"], {
			params: { name: matchInfo.name },
		});
		this.conn = handle.connect() as typeof this.conn;

		this.conn.on("snapshot", (raw: unknown) => {
			const snap = raw as Snapshot;
			const now = Date.now();

			if (this.lastSnapshotTime > 0) {
				this.tickIntervalMs = now - this.lastSnapshotTime;
			}
			this.lastSnapshotTime = now;
			this.latencyMs = Math.max(0, now - snap.serverTime);

			for (const [id, info] of Object.entries(snap.players)) {
				this.playerNames[id] = info.name;
				if (info.name === matchInfo.name && !this.myConnId) {
					this.myConnId = id;
				}
			}

			for (const body of snap.bodies) {
				this.targets[body.id] = body;

				// Create meshes for dynamic bodies we don't have yet (e.g. spawned cubes).
				if (!body.id.startsWith("player-") && !this.dynamicMeshes[body.id]) {
					const size = 0.5; // Default visual size; server controls physics.
					const geo = new THREE.BoxGeometry(size * 2, size * 2, size * 2);
					const mat = new THREE.MeshStandardMaterial({ color: colorFromId(body.id) });
					const mesh = new THREE.Mesh(geo, mat);
					mesh.position.set(body.x, body.y, body.z);
					mesh.castShadow = true;
					mesh.receiveShadow = true;
					this.scene.add(mesh);
					this.dynamicMeshes[body.id] = mesh;
				}
			}

			// Remove stale bodies.
			const currentIds = new Set(snap.bodies.map((b) => b.id));
			for (const id of Object.keys(this.targets)) {
				if (!currentIds.has(id)) {
					delete this.targets[id];
					if (this.dynamicMeshes[id]) {
						this.scene.remove(this.dynamicMeshes[id]);
						delete this.dynamicMeshes[id];
					}
				}
			}

			// Manage player meshes.
			for (const [id, info] of Object.entries(snap.players)) {
				if (!this.playerMeshes[id]) {
					const isMe = id === this.myConnId;
					const geo = new THREE.SphereGeometry(PLAYER_RADIUS, 16, 12);
					const mat = new THREE.MeshStandardMaterial({
						color: isMe ? 0xff4f00 : colorFromId(id),
					});
					const mesh = new THREE.Mesh(geo, mat);
					mesh.castShadow = true;
					this.scene.add(mesh);
					this.playerMeshes[id] = mesh;

					// Name label sprite.
					const label = this.createLabel(info.name);
					this.scene.add(label);
					this.playerLabels[id] = label;
				}
			}

			// Remove disconnected player meshes.
			for (const id of Object.keys(this.playerMeshes)) {
				if (!snap.players[id]) {
					this.scene.remove(this.playerMeshes[id]);
					this.scene.remove(this.playerLabels[id]);
					delete this.playerMeshes[id];
					delete this.playerLabels[id];
					delete this.playerNames[id];
				}
			}
		});

		this.renderer.domElement.addEventListener("click", this.onClick);
		window.addEventListener("keydown", this.onKeyDown);
		window.addEventListener("keyup", this.onKeyUp);
		this.rafId = requestAnimationFrame(this.draw);
	}

	destroy() {
		this.stopped = true;
		cancelAnimationFrame(this.rafId);
		this.renderer.domElement.removeEventListener("click", this.onClick);
		window.removeEventListener("keydown", this.onKeyDown);
		window.removeEventListener("keyup", this.onKeyUp);
		this.conn.dispose().catch(() => {});
		this.renderer.dispose();
		if (this.container.contains(this.renderer.domElement)) {
			this.container.removeChild(this.renderer.domElement);
		}
		if (this.container.contains(this.hudEl)) {
			this.container.removeChild(this.hudEl);
		}
	}

	private createLabel(text: string): THREE.Sprite {
		const canvas = document.createElement("canvas");
		canvas.width = 256;
		canvas.height = 64;
		const ctx = canvas.getContext("2d")!;
		ctx.fillStyle = "#ffffff";
		ctx.font = "bold 28px sans-serif";
		ctx.textAlign = "center";
		ctx.textBaseline = "middle";
		ctx.fillText(text, 128, 32);

		const texture = new THREE.CanvasTexture(canvas);
		const mat = new THREE.SpriteMaterial({ map: texture, depthTest: false });
		const sprite = new THREE.Sprite(mat);
		sprite.scale.set(2, 0.5, 1);
		return sprite;
	}

	private onClick = (e: MouseEvent) => {
		const rect = this.renderer.domElement.getBoundingClientRect();
		const mouse = new THREE.Vector2(
			((e.clientX - rect.left) / rect.width) * 2 - 1,
			-((e.clientY - rect.top) / rect.height) * 2 + 1,
		);
		this.raycaster.setFromCamera(mouse, this.camera);
		const hit = new THREE.Vector3();
		if (this.raycaster.ray.intersectPlane(this.groundPlane, hit)) {
			this.conn.spawnBox({ x: hit.x, z: hit.z }).catch(() => {});
		}
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
		let iz = 0;
		if (this.keys["a"] || this.keys["A"] || this.keys["ArrowLeft"]) ix -= 1;
		if (this.keys["d"] || this.keys["D"] || this.keys["ArrowRight"]) ix += 1;
		if (this.keys["w"] || this.keys["W"] || this.keys["ArrowUp"]) iz -= 1;
		if (this.keys["s"] || this.keys["S"] || this.keys["ArrowDown"]) iz += 1;
		const jump = !!this.keys[" "];

		if (ix !== this.lastIx || iz !== this.lastIz || (jump && !this.lastJump)) {
			this.lastIx = ix;
			this.lastIz = iz;
			this.lastJump = jump;
			this.conn.setInput({ inputX: ix, inputZ: iz, jump }).catch(() => {});
		}
		if (!jump && this.lastJump) {
			this.lastJump = false;
		}
	}

	private draw = () => {
		if (this.stopped) return;

		// Lerp dynamic scene bodies.
		for (const [id, mesh] of Object.entries(this.dynamicMeshes)) {
			const target = this.targets[id];
			if (!target) continue;
			mesh.position.x += (target.x - mesh.position.x) * CORRECTION_ALPHA;
			mesh.position.y += (target.y - mesh.position.y) * CORRECTION_ALPHA;
			mesh.position.z += (target.z - mesh.position.z) * CORRECTION_ALPHA;

			const targetQuat = new THREE.Quaternion(
				target.qx,
				target.qy,
				target.qz,
				target.qw,
			);
			mesh.quaternion.slerp(targetQuat, CORRECTION_ALPHA);
		}

		// Lerp player meshes.
		for (const [id, mesh] of Object.entries(this.playerMeshes)) {
			const target = this.targets[`player-${id}`];
			if (!target) continue;
			mesh.position.x += (target.x - mesh.position.x) * CORRECTION_ALPHA;
			mesh.position.y += (target.y - mesh.position.y) * CORRECTION_ALPHA;
			mesh.position.z += (target.z - mesh.position.z) * CORRECTION_ALPHA;

			// Position label above player.
			const label = this.playerLabels[id];
			if (label) {
				label.position.copy(mesh.position);
				label.position.y += PLAYER_RADIUS + 0.5;
			}
		}

		// Camera follows the local player.
		const myMesh = this.playerMeshes[this.myConnId];
		if (myMesh) {
			const target = myMesh.position;
			this.camera.position.x += (target.x - this.camera.position.x) * 0.05;
			this.camera.position.z += (target.z + 8 - this.camera.position.z) * 0.05;
			this.camera.position.y += (target.y + 6 - this.camera.position.y) * 0.05;
			this.camera.lookAt(target.x, target.y, target.z);
		}

		this.renderer.render(this.scene, this.camera);

		// Update HUD.
		const tps = this.tickIntervalMs > 0 ? (1000 / this.tickIntervalMs).toFixed(1) : "—";
		this.hudEl.textContent = `TPS: ${tps}  Interval: ${this.tickIntervalMs}ms | Latency: ~${this.latencyMs}ms`;

		this.rafId = requestAnimationFrame(this.draw);
	};
}

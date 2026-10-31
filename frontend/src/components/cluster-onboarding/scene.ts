import { useState, useSyncExternalStore } from "react";

/**
 * A tiny tween engine for the cluster map. Every animated thing is an
 * entity with a numeric pose; a component declares the poses it wants and
 * the scene moves each entity there on one shared clock, so a card's
 * travel, the slot it opens, and the stack that shifts to make room all
 * cover the same frames. Entities that disappear from the targets play an
 * exit pose and are then dropped.
 *
 * Nothing here touches CSS layout: callers place entities absolutely from
 * the poses they get back.
 */

export type Pose = Readonly<Record<string, number>>;
export type Phase = "enter" | "present" | "exit";

export interface Entity {
	pose: Pose;
	phase: Phase;
	/** 0..1 along the current tween. */
	progress: number;
}

export interface SceneOptions {
	/** Tween length in ms at time scale 1. */
	duration: number;
	ease: (t: number) => number;
	/** Where a new entity starts before travelling to its target. */
	enter: (key: string, target: Pose) => Pose;
	/** Where a removed entity travels before it is dropped. */
	exit: (key: string, last: Pose) => Pose;
	/** Called once an exiting entity has been dropped. */
	onExited?: (key: string) => void;
}

interface Tween {
	from: Pose;
	to: Pose;
	/** Scene clock reading when the tween started. */
	start: number;
	phase: Phase;
}

const EMPTY: ReadonlyMap<string, Entity> = new Map();

export class Scene {
	timeScale = 1;

	private tweens = new Map<string, Tween>();
	/** Scaled milliseconds; only advances while something is moving. */
	private clock = 0;
	private lastNow: number | undefined;
	private frame = 0;
	private listeners = new Set<() => void>();
	private snapshot: ReadonlyMap<string, Entity> = EMPTY;
	private started = false;

	constructor(public opts: SceneOptions) {}

	/**
	 * Declares where every entity should be. New keys enter, missing keys
	 * exit, changed poses retarget from wherever the entity is right now.
	 * The first call places everything at its target with no animation.
	 */
	setTargets(targets: ReadonlyMap<string, Pose>): void {
		let changed = false;
		for (const [key, to] of targets) {
			const current = this.tweens.get(key);
			if (!current) {
				this.tweens.set(
					key,
					this.started
						? {
								from: this.opts.enter(key, to),
								to,
								start: this.clock,
								phase: "enter",
							}
						: // Already complete: nothing to animate.
							{
								from: to,
								to,
								start: this.clock - this.opts.duration,
								phase: "present",
							},
				);
				changed = true;
			} else if (current.phase === "exit" || !samePose(current.to, to)) {
				this.tweens.set(key, {
					from: this.poseOf(current),
					to,
					start: this.clock,
					phase: "present",
				});
				changed = true;
			}
		}
		for (const [key, current] of this.tweens) {
			if (targets.has(key) || current.phase === "exit") continue;
			const last = this.poseOf(current);
			this.tweens.set(key, {
				from: last,
				to: this.opts.exit(key, last),
				start: this.clock,
				phase: "exit",
			});
			changed = true;
		}
		this.started = true;
		if (changed) {
			// Called during render: refresh the snapshot the caller is about
			// to read, but leave notifying to the next frame.
			this.publish(false);
			this.run();
		}
	}

	subscribe = (listener: () => void): (() => void) => {
		this.listeners.add(listener);
		return () => {
			this.listeners.delete(listener);
			if (this.listeners.size === 0) this.stop();
		};
	};

	getSnapshot = (): ReadonlyMap<string, Entity> => this.snapshot;

	private progressOf(tween: Tween): number {
		return Math.min(1, (this.clock - tween.start) / this.opts.duration);
	}

	private poseOf(tween: Tween): Pose {
		const e = this.opts.ease(this.progressOf(tween));
		const pose: Record<string, number> = {};
		for (const key of Object.keys(tween.to)) {
			const from = tween.from[key] ?? tween.to[key];
			pose[key] = from + (tween.to[key] - from) * e;
		}
		return pose;
	}

	private publish(notify = true): void {
		const next = new Map<string, Entity>();
		for (const [key, tween] of this.tweens) {
			next.set(key, {
				pose: this.poseOf(tween),
				phase: tween.phase,
				progress: this.progressOf(tween),
			});
		}
		this.snapshot = next;
		if (notify) for (const listener of this.listeners) listener();
	}

	private run(): void {
		if (this.frame !== 0 || typeof requestAnimationFrame !== "function") {
			return;
		}
		this.lastNow = undefined;
		this.frame = requestAnimationFrame(this.tick);
	}

	private stop(): void {
		if (this.frame !== 0) cancelAnimationFrame(this.frame);
		this.frame = 0;
	}

	private tick = (now: number): void => {
		this.frame = 0;
		if (this.lastNow !== undefined) {
			this.clock += (now - this.lastNow) * this.timeScale;
		}
		this.lastNow = now;
		let moving = false;
		const exited: string[] = [];
		for (const [key, tween] of this.tweens) {
			const done = this.progressOf(tween) >= 1;
			if (done && tween.phase === "exit") {
				this.tweens.delete(key);
				exited.push(key);
			} else if (done) {
				tween.phase = "present";
			} else {
				moving = true;
			}
		}
		this.publish();
		for (const key of exited) this.opts.onExited?.(key);
		if (moving) this.frame = requestAnimationFrame(this.tick);
	};
}

function samePose(a: Pose, b: Pose): boolean {
	const keys = Object.keys(a);
	if (keys.length !== Object.keys(b).length) return false;
	return keys.every((k) => a[k] === b[k]);
}

/**
 * Subscribes a component to a scene it owns. Targets are declared each
 * render; the scene only reacts to actual changes.
 */
export function useScene(
	targets: ReadonlyMap<string, Pose>,
	opts: SceneOptions,
	timeScale = 1,
): ReadonlyMap<string, Entity> {
	const [scene] = useState(() => new Scene(opts));
	scene.opts = opts;
	scene.timeScale = timeScale;
	scene.setTargets(targets);
	return useSyncExternalStore(
		scene.subscribe,
		scene.getSnapshot,
		scene.getSnapshot,
	);
}

/** Cubic bezier easing, the same curve CSS `cubic-bezier()` would give. */
export function cubicBezier(
	x1: number,
	y1: number,
	x2: number,
	y2: number,
): (t: number) => number {
	const sample = (a: number, b: number, t: number) =>
		(1 - 3 * b + 3 * a) * t * t * t + (3 * b - 6 * a) * t * t + 3 * a * t;
	return (t: number) => {
		if (t <= 0) return 0;
		if (t >= 1) return 1;
		// Solve x(u) = t for u by bisection, then read y(u).
		let lo = 0;
		let hi = 1;
		let u = t;
		for (let i = 0; i < 24; i++) {
			const x = sample(x1, x2, u);
			if (Math.abs(x - t) < 1e-5) break;
			if (x < t) lo = u;
			else hi = u;
			u = (lo + hi) / 2;
		}
		return sample(y1, y2, u);
	};
}

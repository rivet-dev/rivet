import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cubicBezier, type Pose, Scene } from "./scene";

/** Drives the scene's rAF loop by hand, one frame per `step`. */
function harness() {
	let frames: FrameRequestCallback[] = [];
	vi.stubGlobal("requestAnimationFrame", (cb: FrameRequestCallback) => {
		frames.push(cb);
		return frames.length;
	});
	vi.stubGlobal("cancelAnimationFrame", () => {});
	let now = 0;
	return {
		step(ms: number) {
			now += ms;
			const due = frames;
			frames = [];
			for (const cb of due) cb(now);
		},
	};
}

const linear = (t: number) => t;
const targets = (entries: Record<string, Pose>) =>
	new Map(Object.entries(entries));

describe("Scene", () => {
	let clock: ReturnType<typeof harness>;
	beforeEach(() => {
		clock = harness();
	});
	afterEach(() => vi.unstubAllGlobals());

	it("places the first targets without animating", () => {
		const scene = new Scene({
			duration: 100,
			ease: linear,
			enter: () => ({ x: -1 }),
			exit: () => ({ x: -1 }),
		});
		scene.setTargets(targets({ a: { x: 10 } }));
		expect(scene.getSnapshot().get("a")).toMatchObject({
			pose: { x: 10 },
			phase: "present",
		});
	});

	it("moves new, changed, and removed entities on one clock", () => {
		const exited: string[] = [];
		const scene = new Scene({
			duration: 100,
			ease: linear,
			enter: (_k, to) => ({ x: to.x - 100 }),
			exit: (_k, last) => ({ x: last.x + 40 }),
			onExited: (k) => exited.push(k),
		});
		scene.subscribe(() => {});
		scene.setTargets(targets({ a: { x: 0 }, b: { x: 200 } }));
		clock.step(16);
		scene.setTargets(targets({ b: { x: 0 }, c: { x: 200 } }));
		clock.step(16);
		// First frame after a change only primes the clock.
		clock.step(50);
		const mid = scene.getSnapshot();
		expect(mid.get("a")).toMatchObject({ phase: "exit", pose: { x: 20 } });
		expect(mid.get("b")).toMatchObject({
			phase: "present",
			pose: { x: 100 },
		});
		expect(mid.get("c")).toMatchObject({
			phase: "enter",
			pose: { x: 150 },
		});
		clock.step(50);
		const end = scene.getSnapshot();
		expect(end.has("a")).toBe(false);
		expect(exited).toEqual(["a"]);
		expect(end.get("b")?.pose.x).toBe(0);
		expect(end.get("c")).toMatchObject({
			phase: "present",
			pose: { x: 200 },
		});
	});

	it("retargets from the current pose instead of restarting", () => {
		const scene = new Scene({
			duration: 100,
			ease: linear,
			enter: (_k, to) => to,
			exit: (_k, last) => last,
		});
		scene.subscribe(() => {});
		scene.setTargets(targets({ a: { x: 0 } }));
		scene.setTargets(targets({ a: { x: 100 } }));
		clock.step(16);
		clock.step(50);
		// The loop is already running, so no priming frame this time.
		scene.setTargets(targets({ a: { x: 0 } }));
		clock.step(50);
		expect(scene.getSnapshot().get("a")?.pose.x).toBe(25);
	});

	it("honours the time scale", () => {
		const scene = new Scene({
			duration: 100,
			ease: linear,
			enter: (_k, to) => to,
			exit: (_k, last) => last,
		});
		scene.subscribe(() => {});
		scene.timeScale = 0.5;
		scene.setTargets(targets({ a: { x: 0 } }));
		scene.setTargets(targets({ a: { x: 100 } }));
		clock.step(16);
		clock.step(100);
		expect(scene.getSnapshot().get("a")?.pose.x).toBe(50);
	});
});

describe("cubicBezier", () => {
	it("passes through the end points and is monotonic", () => {
		const ease = cubicBezier(0.16, 1, 0.3, 1);
		expect(ease(0)).toBe(0);
		expect(ease(1)).toBe(1);
		let last = 0;
		for (let t = 0.05; t <= 1; t += 0.05) {
			const y = ease(t);
			expect(y).toBeGreaterThanOrEqual(last);
			last = y;
		}
		// Ease-out: well past halfway by a quarter of the time.
		expect(ease(0.25)).toBeGreaterThan(0.7);
	});
});

import { useEffect, useRef, useState } from "react";
import { Button } from "@/components/ui/button";
import { DeploymentMap } from "./deployment-map";
import {
	clusterEndpoints,
	clusterState,
	DEFAULT_SELECTION,
	nextEngineVersion,
	type Rollout,
	rolloutEndMs,
} from "./model";

const ENGINE_VERSION = "3.0.0-alpha.2";
const INITIAL_BUILD = "v12";
/** Far enough in that every node is ready when the lab opens. */
const SETTLED_MS = 60_000;
const TIME_SCALES = [1, 0.5, 0.25, 0.1, 0.05] as const;

/**
 * Standalone harness for the cluster map's motion. The cluster clock and
 * every animation run at the chosen time scale, so a rollout can be stepped
 * through with screenshots and inspected frame by frame.
 */
export function ClusterMapLab() {
	const [timeScale, setTimeScale] = useState<number>(1);
	const [paused, setPaused] = useState(false);
	const [runnerCount, setRunnerCount] = useState(
		DEFAULT_SELECTION.runners.count,
	);
	const [elapsedMs, setElapsedMs] = useState(SETTLED_MS);
	const [rollouts, setRollouts] = useState<Rollout[]>([]);
	const [buildN, setBuildN] = useState(12);
	const [engineVersion, setEngineVersion] = useState(ENGINE_VERSION);

	// Scaled wall clock: the model advances `timeScale` ms per real ms.
	const clock = useRef({ timeScale, paused });
	clock.current = { timeScale, paused };
	useEffect(() => {
		let last = performance.now();
		let frame = 0;
		const tick = (now: number) => {
			const dt = now - last;
			last = now;
			if (!clock.current.paused) {
				setElapsedMs((ms) => ms + dt * clock.current.timeScale);
			}
			frame = requestAnimationFrame(tick);
		};
		frame = requestAnimationFrame(tick);
		return () => cancelAnimationFrame(frame);
	}, []);

	const selection = {
		...DEFAULT_SELECTION,
		runners: { ...DEFAULT_SELECTION.runners, count: runnerCount },
	};
	const { nodes, status } = clusterState(
		selection,
		elapsedMs,
		[],
		rollouts,
		INITIAL_BUILD,
	);
	const rolling = rollouts.some(
		(r) =>
			r.startedMs <= elapsedMs && elapsedMs < rolloutEndMs(selection, r),
	);
	const startRollout = (kind: Rollout["kind"], version: string) =>
		setRollouts((prev) => [
			...prev,
			{ kind, version, startedMs: elapsedMs },
		]);
	const rollRunners = () => {
		const next = buildN + 1;
		setBuildN(next);
		startRollout("runners", `v${next}`);
	};
	const rollControlPlane = () => {
		const next = nextEngineVersion(engineVersion);
		setEngineVersion(next);
		startRollout("control-plane", next);
	};
	const reset = () => {
		setRollouts([]);
		setElapsedMs(SETTLED_MS);
		setBuildN(12);
		setEngineVersion(ENGINE_VERSION);
	};

	return (
		<div className="mx-auto flex max-w-5xl flex-col gap-4 p-6">
			<div className="flex flex-wrap items-center gap-2 text-sm">
				<Button
					size="sm"
					variant="outline"
					onClick={rollRunners}
					disabled={rolling}
					data-lab="roll-runners"
				>
					Roll runners → v{buildN + 1}
				</Button>
				<Button
					size="sm"
					variant="outline"
					onClick={rollControlPlane}
					disabled={rolling}
					data-lab="roll-control-plane"
				>
					Roll control plane → {nextEngineVersion(engineVersion)}
				</Button>
				<Button
					size="sm"
					variant="ghost"
					onClick={() => setPaused((p) => !p)}
					data-lab="pause"
				>
					{paused ? "Play" : "Pause"}
				</Button>
				<Button
					size="sm"
					variant="ghost"
					onClick={reset}
					data-lab="reset"
				>
					Reset
				</Button>
				<label className="ml-auto flex items-center gap-2">
					Runners
					<select
						className="rounded-md border border-border bg-background px-2 py-1"
						value={runnerCount}
						onChange={(e) => setRunnerCount(Number(e.target.value))}
						data-lab="runner-count"
					>
						{Array.from({ length: 8 }, (_, i) => i + 1).map((n) => (
							<option key={n} value={n}>
								{n}
							</option>
						))}
					</select>
				</label>
				<label className="flex items-center gap-2">
					Speed
					<select
						className="rounded-md border border-border bg-background px-2 py-1"
						value={timeScale}
						onChange={(e) => setTimeScale(Number(e.target.value))}
						data-lab="time-scale"
					>
						{TIME_SCALES.map((s) => (
							<option key={s} value={s}>
								{s}×
							</option>
						))}
					</select>
				</label>
				<Button
					size="sm"
					variant="ghost"
					onClick={() =>
						document.documentElement.classList.toggle("dark")
					}
				>
					Theme
				</Button>
			</div>
			<div
				className="font-mono text-xs text-muted-foreground"
				data-lab="clock"
			>
				t={Math.round(elapsedMs)}ms · {status}
			</div>
			<DeploymentMap
				nodes={nodes}
				externalHost={
					new URL(clusterEndpoints(selection).external.url).host
				}
				timeScale={timeScale}
			/>
		</div>
	);
}

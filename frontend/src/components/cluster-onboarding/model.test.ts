import { describe, expect, it } from "vitest";
import {
	clampSize,
	clampRunners,
	type ClusterNode,
	clusterEndpoints,
	clusterState,
	ENGINE_VERSION,
	nextEngineVersion,
	DEMO_CLUSTER_SELECTION,
	detectUseCase,
	formatDiskGb,
	isValidCidr,
	quote,
	selectTier,
} from "./model";

describe("quote", () => {
	it("prices the plan fee plus the runners on the AWS baseline", () => {
		// Team-4 is $120 for 3 × r-40 and covers the control plane; r-20 is
		// $13 per runner node.
		const q = quote({
			...DEMO_CLUSTER_SELECTION,
			tier: "team",
			runners: { size: "r-20", count: 2 },
		});
		expect(q.controlPlane).toMatchObject({ nodes: 3 });
		expect(q.controlPlane.size.id).toBe("r-40");
		expect(q.runnersUsd).toBe(26);
		expect(q.totalUsd).toBe(146);
		expect(q.lines.slice(0, 2)).toEqual([
			{
				label: "Control Plane · Startup-4 · 3 × 1 vCPU / 4 GB",
				amount: 120,
			},
			{
				label: "Actor node compute · 2 × ½ vCPU / 2 GB on AWS",
				amount: 26,
			},
		]);
	});

	it("prices a larger control plane option within the same tier", () => {
		const q = quote({
			...DEMO_CLUSTER_SELECTION,
			tier: "team",
			plan: "team-8",
			runners: { size: "r-20", count: 2 },
		});
		expect(q.controlPlane).toMatchObject({ nodes: 3 });
		expect(q.controlPlane.size.id).toBe("r-80");
		expect(q.planUsd).toBe(220);
		expect(q.totalUsd).toBe(246);
	});

	it("prices a Business option by node count and size", () => {
		const q = quote({
			...DEMO_CLUSTER_SELECTION,
			tier: "enterprise",
			plan: "enterprise-7x-64",
			runners: { size: "r-20", count: 2 },
		});
		expect(q.controlPlane).toMatchObject({ nodes: 7 });
		expect(q.controlPlane.size.id).toBe("r-640");
		expect(q.plan.label).toBe("Business-7x-64");
		expect(q.planUsd).toBe(4500);
	});

	it("falls back to the tier's smallest option when the plan belongs to another tier", () => {
		const q = quote({
			...DEMO_CLUSTER_SELECTION,
			tier: "pro",
			plan: "team-8",
			runners: { size: "r-20", count: 1 },
		});
		expect(q.plan.id).toBe("pro-1");
		expect(q.planUsd).toBe(10);
	});

	it("applies the cloud multiplier to the control plane and runners", () => {
		// Google Cloud is 1.05x: plan 120 * 1.05 = $126; 13 * 1.05 = 13.65
		// -> $14 per node, 3 nodes = $42.
		const q = quote({
			...DEMO_CLUSTER_SELECTION,
			tier: "team",
			cloud: "gcp",
			region: "us-central1",
			runners: { size: "r-20", count: 3 },
		});
		expect(q.planUsd).toBe(126);
		expect(q.runnersUsd).toBe(42);
		expect(q.totalUsd).toBe(168);
	});

	it("includes object storage at 5x the plan's actor storage", () => {
		const team = quote({
			...DEMO_CLUSTER_SELECTION,
			tier: "team",
			runners: { size: "r-20", count: 2 },
		});
		expect(team.objectStorageGb).toBe(500);
		expect(team.lines).toContainEqual({
			label: "Object storage · 500 GB",
			amount: "Included",
		});
		// Included lines must not change the total.
		expect(team.totalUsd).toBe(146);
	});
});

describe("selectTier", () => {
	it("resets the control plane option and runners to the tier defaults", () => {
		const next = selectTier(
			{
				...DEMO_CLUSTER_SELECTION,
				tier: "enterprise",
				plan: "enterprise-9x-64",
				runners: { size: "r-640", count: 40 },
			},
			"pro",
		);
		expect(next.tier).toBe("pro");
		expect(next.plan).toBe("pro-1");
		expect(next.runners).toEqual({ size: "r-10", count: 1 });
		// Multi-zone plans start with one runner per zone on 2+ vCPU nodes.
		expect(selectTier(next, "team").runners).toEqual({
			size: "r-80",
			count: 3,
		});
		expect(selectTier(next, "enterprise").runners).toEqual({
			size: "r-80",
			count: 3,
		});
	});
});

describe("clampRunners", () => {
	it("clamps the size and count to what the plan allows", () => {
		expect(clampRunners("pro", { size: "r-160", count: 5 })).toEqual({
			size: "r-80",
			count: 3,
		});
		expect(clampRunners("team", { size: "r-160", count: 5 })).toEqual({
			size: "r-160",
			count: 5,
		});
		// Multi-zone plans need at least 3 runners.
		expect(clampRunners("team", { size: "r-20", count: 1 })).toEqual({
			size: "r-20",
			count: 3,
		});
		// Business has no runner limit.
		expect(
			clampRunners("enterprise", { size: "r-1280", count: 500 }),
		).toEqual({
			size: "r-1280",
			count: 500,
		});
	});
});

describe("clampSize", () => {
	it("keeps sizes the plan allows and clamps the rest to the plan max", () => {
		expect(clampSize("pro", "r-40")).toBe("r-40");
		expect(clampSize("pro", "r-80")).toBe("r-80");
		expect(clampSize("pro", "r-160")).toBe("r-80");
	});
});

describe("formatDiskGb", () => {
	it("switches to TB at 1000 GB", () => {
		expect(formatDiskGb(960)).toBe("960 GB");
		expect(formatDiskGb(1600)).toBe("1.6 TB");
		expect(formatDiskGb(3000)).toBe("3 TB");
	});
});

describe("isValidCidr", () => {
	it("accepts IPv4 addresses with or without a prefix", () => {
		expect(isValidCidr("10.20.0.0/16")).toBe(true);
		expect(isValidCidr("203.0.113.42")).toBe(true);
		expect(isValidCidr("0.0.0.0/0")).toBe(true);
	});

	it("rejects out-of-range octets and prefixes", () => {
		expect(isValidCidr("256.0.0.1")).toBe(false);
		expect(isValidCidr("10.0.0.0/33")).toBe(false);
		expect(isValidCidr("10.0.0/8")).toBe(false);
		expect(isValidCidr("not-an-ip")).toBe(false);
	});
});

describe("detectUseCase", () => {
	it("picks the use case with the most keyword hits", () => {
		expect(
			detectUseCase("A browser .io game with lobbies and matchmaking").id,
		).toBe("multiplayer-game");
		expect(
			detectUseCase("Team chat with channels, DMs and presence").id,
		).toBe("chat");
	});

	it("falls back to custom when nothing matches", () => {
		expect(detectUseCase("").id).toBe("custom");
		expect(
			detectUseCase("inventory reconciliation for warehouses").id,
		).toBe("custom");
	});
});

describe("clusterEndpoints", () => {
	it("derives the object storage bucket and region from the cluster", () => {
		const e = clusterEndpoints({
			...DEMO_CLUSTER_SELECTION,
			name: "Chat Prod",
			region: "aws:us-east-1",
		});
		expect(e.external.url).toBe("https://api.chat-prod.rivet.run");
		expect(e.objectStorage.url).toBe("https://s3.chat-prod.rivet.run");
		expect(e.objectStorage.bucket).toBe("chat-prod");
		expect(e.objectStorage.accessKeyId).toMatch(/^RVT[A-Z2-7]{17}$/);
		expect(e.objectStorage.secretAccessKey).toHaveLength(40);
	});
});

describe("nextEngineVersion", () => {
	it("increments the pre-release number", () => {
		expect(nextEngineVersion("3.0.0-alpha.2")).toBe("3.0.0-alpha.3");
		expect(nextEngineVersion("3.0.0-alpha.9")).toBe("3.0.0-alpha.10");
	});
});

describe("clusterState", () => {
	const selection = {
		...DEMO_CLUSTER_SELECTION,
		runners: { size: "r-20" as const, count: 2 },
	};
	const kinds = (nodes: ClusterNode[]) =>
		nodes.map((n) => `${n.pool ? "runner" : "cp"}:${n.status}`);

	it("brings the control plane up before the runners", () => {
		const at = (ms: number) => clusterState(selection, ms);

		expect(at(0).status).toBe("provisioning");
		expect(at(0).nodes.every((n) => n.status === "provisioning")).toBe(
			true,
		);

		const mid = at(3000);
		expect(kinds(mid.nodes)).toEqual([
			"cp:ready",
			"cp:ready",
			"cp:ready",
			"runner:provisioning",
			"runner:provisioning",
		]);
		expect(mid.status).toBe("provisioning");

		const done = at(60_000);
		expect(done.status).toBe("active");
		expect(done.nodes.filter((n) => n.pool === "default")).toHaveLength(2);
		expect(done.nodes.find((n) => !n.pool)?.size).toBe("1 vCPU / 4 GB");
		expect(done.nodes.find((n) => n.pool)?.size).toBe("½ vCPU / 2 GB");
	});

	it("provisions a runner added later from the time it was added", () => {
		const scaled = {
			...selection,
			runners: { ...selection.runners, count: 3 },
		};
		// Added at t=60s; polled 1s later: still provisioning.
		expect(
			kinds(clusterState(scaled, 61_000, [0, 0, 60_000]).nodes),
		).toEqual([
			"cp:ready",
			"cp:ready",
			"cp:ready",
			"runner:ready",
			"runner:ready",
			"runner:provisioning",
		]);
		expect(
			clusterState(scaled, 63_000, [0, 0, 60_000]).nodes.at(-1)?.status,
		).toBe("ready");
	});

	it("rolls a deploy across runners with 25% surge", () => {
		// 2 runners: surge rounds up to 1, so two batches of one node each.
		const rollout = {
			kind: "runners" as const,
			version: "v13",
			startedMs: 60_000,
		};
		const at = (ms: number) =>
			clusterState(selection, ms, [], [rollout], "v12");
		const runners = (ms: number) =>
			at(ms)
				.nodes.filter((n) => n.pool)
				.map((n) => `${n.status}:${n.version}`);

		expect(runners(59_000)).toEqual(["ready:v12", "ready:v12"]);
		// A third node comes up on v13 beside the two old ones.
		expect(runners(60_500)).toEqual([
			"ready:v12",
			"ready:v12",
			"provisioning:v13",
		]);
		expect(at(60_500).status).toBe("updating");
		// Once it is ready the first old node is terminated.
		expect(runners(62_500)).toEqual([
			"terminating:v12",
			"ready:v12",
			"ready:v13",
		]);
		// Second batch: the old node is gone and the next new one comes up.
		expect(runners(63_500)).toEqual([
			"ready:v12",
			"ready:v13",
			"provisioning:v13",
		]);
		expect(runners(70_000)).toEqual(["ready:v13", "ready:v13"]);
		expect(at(70_000).status).toBe("active");
		// Control plane keeps its engine version throughout.
		expect(
			at(62_500)
				.nodes.filter((n) => !n.pool)
				.every(
					(n) => n.status === "ready" && n.version === ENGINE_VERSION,
				),
		).toBe(true);
	});

	it("rolls the control plane in place, highest ordinal first", () => {
		const rollout = {
			kind: "control-plane" as const,
			version: "3.0.0-alpha.3",
			startedMs: 60_000,
		};
		const cp = (ms: number) =>
			clusterState(selection, ms, [], [rollout])
				.nodes.filter((n) => !n.pool)
				.map((n) => `${n.status}:${n.version}`);

		// Node count never changes; the last node goes first.
		expect(cp(60_500)).toEqual([
			"ready:3.0.0-alpha.2",
			"ready:3.0.0-alpha.2",
			"terminating:3.0.0-alpha.2",
		]);
		expect(cp(61_500)).toEqual([
			"ready:3.0.0-alpha.2",
			"ready:3.0.0-alpha.2",
			"provisioning:3.0.0-alpha.3",
		]);
		expect(cp(62_500)).toEqual([
			"ready:3.0.0-alpha.2",
			"terminating:3.0.0-alpha.2",
			"ready:3.0.0-alpha.3",
		]);
		expect(cp(70_000)).toEqual([
			"ready:3.0.0-alpha.3",
			"ready:3.0.0-alpha.3",
			"ready:3.0.0-alpha.3",
		]);
		expect(clusterState(selection, 70_000, [], [rollout]).status).toBe(
			"active",
		);
	});
});

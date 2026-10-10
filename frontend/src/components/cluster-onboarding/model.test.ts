import { describe, expect, it } from "vitest";
import {
	clampSize,
	clampRunners,
	type ClusterNode,
	clusterEndpoints,
	clusterState,
	ENGINE_VERSION,
	nextEngineVersion,
	DEFAULT_SELECTION,
	detectUseCase,
	isValidCidr,
	quote,
} from "./model";

describe("quote", () => {
	it("prices the plan fee plus the runners on the AWS baseline", () => {
		// Team plan is $200 and covers the control plane; R-10 is $8/node.
		const q = quote({
			...DEFAULT_SELECTION,
			tier: "team",
			runners: { size: "r-10", count: 2 },
		});
		expect(q.controlPlane).toMatchObject({ nodes: 3 });
		expect(q.controlPlane.size.id).toBe("r-20");
		expect(q.runnersUsd).toBe(16);
		expect(q.totalUsd).toBe(216);
		expect(q.lines.slice(0, 2)).toEqual([
			{ label: "Control plane · 3 × R-20 · Team plan", amount: 200 },
			{ label: "Runner compute · 2 × R-10 on AWS", amount: 16 },
		]);
	});

	it("applies the cloud multiplier to runners only", () => {
		// Hetzner is 0.7x: 8 * 0.7 = 5.6 -> $6 per node, 3 nodes = $18.
		const q = quote({
			...DEFAULT_SELECTION,
			tier: "team",
			cloud: "hetzner",
			region: "fsn1",
			runners: { size: "r-10", count: 3 },
		});
		expect(q.runnersUsd).toBe(18);
		expect(q.totalUsd).toBe(218);
	});

	it("runs Free with its single runner included and the plan's 5 GB", () => {
		const q = quote({
			...DEFAULT_SELECTION,
			tier: "free",
			runners: { size: "r-5", count: 1 },
		});
		expect(q.controlPlane.nodes).toBe(1);
		expect(q.storageGb).toBe(5);
		expect(q.totalUsd).toBe(0);
		expect(q.lines[1].amount).toBe("Included");
	});

	it("includes object storage at 5x the plan's actor storage", () => {
		const team = quote({
			...DEFAULT_SELECTION,
			tier: "team",
			runners: { size: "r-10", count: 2 },
		});
		expect(team.objectStorageGb).toBe(500);
		expect(team.lines).toContainEqual({
			label: "Object storage · 500 GB",
			amount: "Included",
		});
		// Included lines must not change the total.
		expect(team.totalUsd).toBe(216);
	});

	it("bills BYOC runners to the customer's cloud", () => {
		const q = quote({
			...DEFAULT_SELECTION,
			cloud: "byoc",
			region: "aws:us-east-1",
		});
		expect(q.runnersUsd).toBe(0);
		expect(q.lines[1].amount).toBe("Billed by your cloud");
		expect(q.totalUsd).toBe(200);
	});
});

describe("clampRunners", () => {
	it("clamps the size and count to what the plan allows", () => {
		expect(clampRunners("pro", { size: "r-80", count: 5 })).toEqual({
			size: "r-40",
			count: 3,
		});
		expect(clampRunners("team", { size: "r-80", count: 5 })).toEqual({
			size: "r-80",
			count: 5,
		});
		expect(clampRunners("free", { size: "r-10", count: 0 })).toEqual({
			size: "r-5",
			count: 1,
		});
	});
});

describe("clampSize", () => {
	it("keeps sizes the plan allows and clamps the rest to the plan max", () => {
		expect(clampSize("pro", "r-20")).toBe("r-20");
		expect(clampSize("pro", "r-40")).toBe("r-40");
		expect(clampSize("pro", "r-80")).toBe("r-40");
		expect(clampSize("free", "r-10")).toBe("r-5");
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
			...DEFAULT_SELECTION,
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
		...DEFAULT_SELECTION,
		runners: { size: "r-10" as const, count: 2 },
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
		expect(done.nodes.find((n) => !n.pool)?.size).toBe("R-20");
		expect(done.nodes.find((n) => n.pool)?.size).toBe("R-10");
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

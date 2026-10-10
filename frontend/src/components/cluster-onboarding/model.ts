// Pure functions behind the mock: pricing, use-case detection, generated
// endpoints, and the prompts handed to coding agents. No React in here so the
// stories and tests can drive it directly.

import {
	AGENT_CLIENTS,
	BILLING,
	type Cloud,
	type CloudId,
	CLOUDS,
	type NodeSize,
	NODE_SIZES,
	type SizeId,
	type Tier,
	type TierId,
	TIERS,
	type UseCase,
	USE_CASES,
	type UseCaseId,
} from "./catalog";

/** The runner compute the user sizes and scales; the control plane is set by the plan. */
export interface RunnerConfig {
	size: SizeId;
	count: number;
}

export interface ClusterSelection {
	name: string;
	tier: TierId;
	cloud: CloudId;
	region: string;
	runners: RunnerConfig;
	/** CIDR blocks allowed to connect. */
	allowlist: string[];
}

export const DEFAULT_SELECTION: ClusterSelection = {
	name: "test-cluster-3",
	tier: "team",
	cloud: "aws",
	region: "us-east-1",
	runners: { size: "r-10", count: 2 },
	allowlist: ["0.0.0.0/0"],
};

export const getTier = (id: TierId): Tier => {
	const tier = TIERS.find((t) => t.id === id);
	if (!tier) throw new Error(`unknown tier: ${id}`);
	return tier;
};

export const getSize = (id: SizeId): NodeSize => {
	const size = NODE_SIZES.find((s) => s.id === id);
	if (!size) throw new Error(`unknown size: ${id}`);
	return size;
};

export const getCloud = (id: CloudId): Cloud => {
	const cloud = CLOUDS.find((c) => c.id === id);
	if (!cloud) throw new Error(`unknown cloud: ${id}`);
	return cloud;
};

/** Whether a runner node size can be ordered on a tier. Sizes are ordered by capacity. */
export function isSizeAvailable(tier: TierId, size: SizeId): boolean {
	const max = NODE_SIZES.findIndex((s) => s.id === getTier(tier).maxSize);
	const idx = NODE_SIZES.findIndex((s) => s.id === size);
	return idx <= max;
}

/** Largest size on the tier at or below the requested one. */
export function clampSize(tier: TierId, size: SizeId): SizeId {
	return isSizeAvailable(tier, size) ? size : getTier(tier).maxSize;
}

/** The closest runner config the tier allows. */
export function clampRunners(
	tier: TierId,
	runners: RunnerConfig,
): RunnerConfig {
	return {
		size: clampSize(tier, runners.size),
		count: Math.min(Math.max(runners.count, 1), getTier(tier).maxRunners),
	};
}

/** Monthly price of one runner node of a size on a cloud, whole dollars. */
export function nodeMonthlyUsd(size: NodeSize, cloud: Cloud): number {
	return Math.round(size.monthlyUsd * cloud.multiplier);
}

/**
 * Monthly price of the runner pool. Free includes its single runner; BYOC
 * compute is billed by the customer's cloud.
 */
export function runnersMonthlyUsd(
	tier: Tier,
	runners: RunnerConfig,
	cloud: Cloud,
): number {
	if (tier.id === "free" || cloud.id === "byoc") return 0;
	return runners.count * nodeMonthlyUsd(getSize(runners.size), cloud);
}

export interface QuoteLine {
	label: string;
	/** Dollar amount, or a non-numeric note like "Included". */
	amount: number | string;
}

export interface Quote {
	tier: Tier;
	cloud: Cloud;
	/** Core engine nodes, preconfigured by the plan. */
	controlPlane: { nodes: number; size: NodeSize };
	/** Runner nodes, sized and scaled by the user. */
	runners: { count: number; size: NodeSize };
	/** Monthly price of the runner pool alone. */
	runnersUsd: number;
	storageGb: number;
	objectStorageGb: number;
	egressGb: number;
	lines: QuoteLine[];
	totalUsd: number;
}

export function quote(selection: ClusterSelection): Quote {
	const tier = getTier(selection.tier);
	const cloud = getCloud(selection.cloud);
	const runnerSize = getSize(selection.runners.size);
	const isFree = tier.id === "free";
	const isByoc = cloud.id === "byoc";

	const runnersUsd = runnersMonthlyUsd(tier, selection.runners, cloud);
	const storageGb = tier.storageGb;
	const cp = tier.controlPlane;

	const cpSize = getSize(cp.size);
	const lines: QuoteLine[] = [
		{
			label: `Control plane · ${cp.nodes} × ${cpSize.label} · ${tier.label} plan`,
			amount: isFree ? "Free" : tier.monthlyUsd,
		},
		{
			label: `Runner compute · ${selection.runners.count} × ${runnerSize.label} on ${cloud.label}`,
			amount: isByoc
				? "Billed by your cloud"
				: isFree
					? "Included"
					: runnersUsd,
		},
		{ label: `Actor storage · ${storageGb} GB`, amount: "Included" },
		{
			label: `Object storage · ${storageGb * BILLING.objectStorageMultiplier} GB`,
			amount: "Included",
		},
		{
			label: `Egress · ${BILLING.includedEgressGb} GB`,
			amount: "Included",
		},
	];

	const totalUsd = lines.reduce(
		(sum, line) =>
			sum + (typeof line.amount === "number" ? line.amount : 0),
		0,
	);

	return {
		tier,
		cloud,
		controlPlane: { nodes: cp.nodes, size: cpSize },
		runners: { count: selection.runners.count, size: runnerSize },
		runnersUsd,
		storageGb,
		objectStorageGb: storageGb * BILLING.objectStorageMultiplier,
		egressGb: BILLING.includedEgressGb,
		lines,
		totalUsd,
	};
}

export function formatVcpu(value: number): string {
	if (Number.isInteger(value)) return `${value}`;
	// Sizes only use power-of-two fractions, so a denominator search is exact.
	for (const d of [2, 4, 8, 16]) {
		if (Number.isInteger(value * d)) return `${value * d}/${d}`;
	}
	return value.toFixed(2);
}

export function formatMemory(mb: number): string {
	return mb >= 1024
		? `${(mb / 1024).toFixed(mb % 1024 ? 1 : 0)} GB`
		: `${mb} MB`;
}

export function formatUsd(value: number): string {
	return `$${value.toLocaleString("en-US")}`;
}

/** Valid IPv4 CIDR, e.g. 10.0.0.0/8. Bare addresses are accepted as /32. */
export function isValidCidr(value: string): boolean {
	const [ip, bits] = value.trim().split("/");
	const octets = ip.split(".");
	if (octets.length !== 4) return false;
	if (!octets.every((o) => /^\d{1,3}$/.test(o) && Number(o) <= 255)) {
		return false;
	}
	if (bits === undefined) return true;
	return /^\d{1,2}$/.test(bits) && Number(bits) <= 32;
}

/** Picks the best matching use case for free text, or "custom". */
export function detectUseCase(text: string): UseCase {
	const words = text.toLowerCase();
	let best: { useCase: UseCase; score: number } | null = null;
	for (const useCase of USE_CASES) {
		const score = useCase.keywords.filter((k) => words.includes(k)).length;
		if (score > 0 && (!best || score > best.score)) {
			best = { useCase, score };
		}
	}
	return best?.useCase ?? getUseCase("custom");
}

export const getUseCase = (id: UseCaseId): UseCase => {
	const useCase = USE_CASES.find((u) => u.id === id);
	if (!useCase) throw new Error(`unknown use case: ${id}`);
	return useCase;
};

export interface ClusterEndpoints {
	external: { url: string };
	internal: { url: string };
	/** One admin JWT for the whole cluster, valid on both endpoints. */
	adminToken: string;
	/** S3-compatible object storage scoped to this cluster. */
	objectStorage: {
		url: string;
		bucket: string;
		region: string;
		accessKeyId: string;
		secretAccessKey: string;
	};
	mcpUrl: string;
}

/** Deterministic fake S3 credentials for the mock. */
function fakeKey(seed: string, length: number, alphabet: string): string {
	let out = "";
	for (let i = 0; i < length; i++) {
		out +=
			alphabet[
				(seed.charCodeAt(i % seed.length) * (i + 11)) % alphabet.length
			];
	}
	return out;
}

/** Deterministic fake JWT so the mock looks right without leaking anything. */
function fakeJwt(seed: string): string {
	const header = btoa(JSON.stringify({ alg: "EdDSA", typ: "JWT" })).replace(
		/=+$/,
		"",
	);
	const payload = btoa(
		JSON.stringify({ sub: seed, scope: "admin", iss: "rivet.dev" }),
	).replace(/=+$/, "");
	let sig = "";
	for (let i = 0; i < 43; i++) {
		sig +=
			"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_"[
				(seed.charCodeAt(i % seed.length) * (i + 7)) % 64
			];
	}
	return `${header}.${payload}.${sig}`;
}

export function clusterEndpoints(
	selection: ClusterSelection,
): ClusterEndpoints {
	const slug = selection.name.toLowerCase().replace(/[^a-z0-9-]/g, "-");
	const region = selection.region.replace(/^(aws|gcp):/, "");
	return {
		external: { url: `https://api.${slug}.rivet.run` },
		internal: { url: `http://api.${slug}.internal.rivet.run:6420` },
		adminToken: fakeJwt(`${slug}:admin`),
		objectStorage: {
			url: `https://s3.${slug}.rivet.run`,
			bucket: slug,
			region,
			accessKeyId: `RVT${fakeKey(`${slug}:s3:id`, 17, "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567")}`,
			secretAccessKey: fakeKey(
				`${slug}:s3:secret`,
				40,
				"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789+/",
			),
		},
		mcpUrl: "https://mcp.rivet.dev/mcp",
	};
}

/**
 * Per-client install instructions, modeled on Cloudflare's agent-setup
 * prompt. Published at a stable URL so the agent can re-verify it.
 */
export const SETUP_PROMPT_URL = "https://rivet.dev/agent-setup/prompt.md";

/**
 * The one line the user pastes into their agent. Everything else (skills,
 * MCP server, CLI) lives in the file at SETUP_PROMPT_URL, like Cloudflare's
 * agent-setup/prompt.md, so the dashboard never has to explain per-client
 * install steps.
 */
export const SETUP_PROMPT = `Fetch and execute the appropriate instructions to set me up for Rivet from ${SETUP_PROMPT_URL}`;

export const SKILLS_INSTALL =
	"npx -y skills add rivet-dev/skills --skill '*' --yes --global";
export const CLI_INSTALL = "npm install -g @rivetkit/cli && rivet login";

/**
 * The whole file behind SETUP_PROMPT_URL. Sets the agent up (skills, MCP, CLI)
 * and then hands it the project flow: ask the user what to build, run against
 * `rivet dev` first, deploy to this cluster, build a dashboard. The
 * dashboard never needs a "what do you want to build" box because the agent
 * asks.
 */
export function setupPrompt(
	selection: ClusterSelection,
	endpoints: ClusterEndpoints,
): string {
	const clients = AGENT_CLIENTS.map(
		(c) =>
			`### ${c.label}\n\n\`\`\`\n${c.mcpInstall(endpoints.mcpUrl)}\n\`\`\``,
	).join("\n\n");
	const patterns = USE_CASES.filter((u) => u.id !== "custom")
		.map(
			(u) =>
				`- ${u.label}: ${u.actors.map((a) => `${a.name} (\`${a.key}\`)`).join(", ")}`,
		)
		.join("\n");
	return [
		"These are official instructions from Rivet to set up an AI development environment for building and deploying on Rivet. Complete the setup steps yourself by running the commands directly. Do not ask the user to run them.",
		"",
		"- Install the Rivet skills",
		"- Register the Rivet MCP server for this cluster",
		"- Install the Rivet CLI",
		"- Then ask the user what they want to build, start locally, deploy, and build them a dashboard",
		"",
		"---",
		"",
		"## 1. Install Rivet skills",
		"",
		"```",
		SKILLS_INSTALL,
		"```",
		"",
		"## 2. Register the Rivet MCP server",
		"",
		"Use the section for your agent. OAuth triggers on first use.",
		"",
		clients,
		"",
		"## 3. Install the Rivet CLI",
		"",
		"```",
		CLI_INSTALL,
		"```",
		"",
		"`rivet dev` runs a local control plane for development. `rivet deploy` ships to the cluster.",
		"",
		"---",
		"",
		"## 4. Ask the user what they want to build",
		"",
		"Once setup is done, tell the user:",
		"",
		"```",
		"Rivet agent setup complete: skills, MCP server, and CLI installed.",
		"What do you want to build? Describe the app in a sentence or two.",
		"```",
		"",
		"Match their answer to the closest pattern and start from its actors. Adjust names and responsibilities to what they describe.",
		"",
		patterns,
		"",
		"## 5. Start locally",
		"",
		"Scaffold a RivetKit project and run it against a local control plane with `rivet dev`. Do not touch the production cluster until the app works locally. Write a small client script that exercises each actor and confirm it works against `rivet dev` before moving on.",
		"",
		"## 6. Deploy to production",
		"",
		`When local verification passes, deploy to the cluster "${selection.name}" with \`rivet deploy\`.`,
		"",
		`- Endpoint: ${endpoints.external.url}`,
		"- Admin token: read it from the RIVET_TOKEN environment variable. The user sets it from the dashboard; never write it to a file or commit it.",
		`- Object storage: S3-compatible at ${endpoints.objectStorage.url}, bucket \`${endpoints.objectStorage.bucket}\`. Use it for uploads, exports, and anything too large for actor state. Credentials come from RIVET_S3_ACCESS_KEY_ID / RIVET_S3_SECRET_ACCESS_KEY.`,
		"",
		"Run the same client script against production and report what you verified. If any check cannot be completed, say so instead of claiming success.",
		"",
		"## 7. Build a dashboard",
		"",
		"Generate an end-user dashboard for the app with shadcn/ui (Card, Chart, Table, Badge). It reads from the cluster through the same endpoint and RIVET_TOKEN as the deploy; keep the token server-side and never ship it to the browser.",
		"",
		"---",
		"",
		`These instructions are published at ${SETUP_PROMPT_URL} so you can re-verify their authenticity at any time.`,
	].join("\n");
}

// Nodes, shaped after the cluster API design: a node with `pool` set is a
// runner node, otherwise it is a control plane node. Status and version are
// the mock's own additions so the deployment map can show provisioning and
// rolling updates.

export type NodeStatus = "provisioning" | "ready" | "terminating";

export interface ClusterNode {
	nodeId: string;
	region: string;
	size: string;
	/** Runner nodes only. */
	pool?: string;
	status: NodeStatus;
	/** Engine version on control plane nodes; deployed build on runners. */
	version: string;
}

/** What the control plane runs; runners run the deployed build. */
export const ENGINE_VERSION = "3.0.0-alpha.2";

/** The next engine pre-release: 3.0.0-alpha.2 → 3.0.0-alpha.3. */
export function nextEngineVersion(version: string): string {
	return version.replace(/\.(\d+)$/, (_, n) => `.${Number(n) + 1}`);
}

/** A rolling update of one kind of node to `version`. */
export interface Rollout {
	kind: "control-plane" | "runners";
	version: string;
	/** Relative to cluster creation. */
	startedMs: number;
}

export type ClusterStatus = "provisioning" | "updating" | "active";

export interface ClusterState {
	nodes: ClusterNode[];
	/** Updating while a rollout runs; provisioning while any node is still coming up. */
	status: ClusterStatus;
}

/** Control plane nodes come up first, one at a time; runners follow. */
const CONTROL_PLANE_READY_MS = (i: number) => 1200 + i * 700;
const RUNNER_READY_MS = (i: number) => 3400 + i * 800;
/** A runner added by scaling up later is ready this long after it was added. */
const SCALED_RUNNER_READY_MS = 2500;
/**
 * The control plane rolls like a StatefulSet: one node at a time, in place,
 * highest ordinal first. Each node is torn down and brought back on the new
 * version before the next one starts.
 */
const CP_TERMINATE_MS = 800;
const CP_START_MS = 1600;
const CP_NODE_MS = CP_TERMINATE_MS + CP_START_MS;
/**
 * Runners roll like a Deployment with maxSurge 25% and no unavailability:
 * a batch of new nodes comes up beside the old ones, and only once they are
 * ready does the same number of old nodes go away.
 */
export const MAX_SURGE = 0.25;
const RUNNER_START_MS = 2000;
const RUNNER_TERMINATE_MS = 1000;
const RUNNER_BATCH_MS = RUNNER_START_MS + RUNNER_TERMINATE_MS;

/** How many extra runners a rollout adds at a time (surge rounds up, like Kubernetes). */
export function surgeCount(count: number): number {
	return Math.max(1, Math.ceil(count * MAX_SURGE));
}

/** When a rollout started on a cluster of this shape is finished. */
export function rolloutEndMs(
	selection: ClusterSelection,
	rollout: Rollout,
): number {
	const q = quote(selection);
	if (rollout.kind === "control-plane") {
		return rollout.startedMs + q.controlPlane.nodes * CP_NODE_MS;
	}
	const batches = Math.ceil(q.runners.count / surgeCount(q.runners.count));
	return rollout.startedMs + batches * RUNNER_BATCH_MS;
}

/** The latest rollout of a kind that has started, and the version before it. */
function currentRollout(
	rollouts: Rollout[],
	kind: Rollout["kind"],
	elapsedMs: number,
): { current?: Rollout; previous?: string } {
	const started = rollouts.filter(
		(r) => r.kind === kind && r.startedMs <= elapsedMs,
	);
	return { current: started.at(-1), previous: started.at(-2)?.version };
}

/**
 * The cluster's nodes and status `elapsedMs` after creation. Everything is
 * derived from the selection, the rollouts, and the elapsed time so a poll
 * can recompute it.
 *
 * `runnerAddedMs[i]` is when runner `i` was added, relative to cluster
 * creation; omitted runners were there from the start. `initialBuild` is
 * what runners ran before any rollout.
 */
export function clusterState(
	selection: ClusterSelection,
	elapsedMs: number,
	runnerAddedMs: number[] = [],
	rollouts: Rollout[] = [],
	initialBuild = "v1",
): ClusterState {
	const slug = selection.name.toLowerCase().replace(/[^a-z0-9-]/g, "-");
	const region = selection.region.replace(/^(aws|gcp):/, "");
	const q = quote(selection);
	const id = (kind: string, i: number) =>
		`${kind}-${fakeKey(`${(i + 1) * 7919}${kind}${slug}`, 6, "abcdefghijklmnopqrstuvwxyz0123456789")}`;

	const cp = currentRollout(rollouts, "control-plane", elapsedMs);
	const cpOld = cp.previous ?? ENGINE_VERSION;
	const cpCount = q.controlPlane.nodes;
	const controlPlane: ClusterNode[] = Array.from(
		{ length: cpCount },
		(_, i) => {
			const base = {
				nodeId: id("cp", i),
				region,
				size: q.controlPlane.size.label,
			};
			if (elapsedMs < CONTROL_PLANE_READY_MS(i)) {
				return { ...base, status: "provisioning", version: cpOld };
			}
			if (!cp.current)
				return { ...base, status: "ready", version: cpOld };
			const start = cp.current.startedMs + (cpCount - 1 - i) * CP_NODE_MS;
			if (elapsedMs < start) {
				return { ...base, status: "ready", version: cpOld };
			}
			if (elapsedMs < start + CP_TERMINATE_MS) {
				return { ...base, status: "terminating", version: cpOld };
			}
			if (elapsedMs < start + CP_NODE_MS) {
				return {
					...base,
					status: "provisioning",
					version: cp.current.version,
				};
			}
			return { ...base, status: "ready", version: cp.current.version };
		},
	);

	const rn = currentRollout(rollouts, "runners", elapsedMs);
	const rnOld = rn.previous ?? initialBuild;
	const count = q.runners.count;
	const surge = surgeCount(count);
	const runnerBase = (i: number, version: string) => ({
		nodeId: id(`wk-${version}`, i),
		region,
		size: q.runners.size.label,
		pool: "default",
	});
	const readyMs = (i: number) => {
		const addedMs = runnerAddedMs[i] ?? 0;
		return addedMs === 0
			? RUNNER_READY_MS(i)
			: addedMs + SCALED_RUNNER_READY_MS;
	};
	// Runners that predate the rollout: old nodes first, then the new ones
	// coming up beside them, so the old version's stack is listed first.
	const inRollout = (i: number) =>
		rn.current !== undefined &&
		(runnerAddedMs[i] ?? 0) <= rn.current.startedMs;
	const batchStart = (i: number) =>
		(rn.current?.startedMs ?? 0) + Math.floor(i / surge) * RUNNER_BATCH_MS;
	const oldRunners: ClusterNode[] = [];
	const newRunners: ClusterNode[] = [];
	for (let i = 0; i < count; i++) {
		if (!inRollout(i)) {
			// Added after the rollout started (or no rollout): comes up on
			// whatever the current build is.
			const version = rn.current?.version ?? rnOld;
			oldRunners.push({
				...runnerBase(i, version),
				status: elapsedMs < readyMs(i) ? "provisioning" : "ready",
				version,
			});
			continue;
		}
		const start = batchStart(i);
		if (elapsedMs < start + RUNNER_START_MS) {
			oldRunners.push({
				...runnerBase(i, rnOld),
				status: elapsedMs < readyMs(i) ? "provisioning" : "ready",
				version: rnOld,
			});
		} else if (elapsedMs < start + RUNNER_BATCH_MS) {
			oldRunners.push({
				...runnerBase(i, rnOld),
				status: "terminating",
				version: rnOld,
			});
		}
		if (rn.current && elapsedMs >= start) {
			newRunners.push({
				...runnerBase(i, rn.current.version),
				status:
					elapsedMs < start + RUNNER_START_MS
						? "provisioning"
						: "ready",
				version: rn.current.version,
			});
		}
	}

	const nodes = [...controlPlane, ...oldRunners, ...newRunners];
	const rolling = rollouts.some(
		(r) =>
			r.startedMs <= elapsedMs && elapsedMs < rolloutEndMs(selection, r),
	);
	const status: ClusterStatus = rolling
		? "updating"
		: nodes.some((n) => n.status === "provisioning")
			? "provisioning"
			: "active";
	return { nodes, status };
}

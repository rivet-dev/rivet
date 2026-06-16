import { computeCostPerSecond } from "@/content/billing";
import type { MetricsColumnar } from "./types";

// Normalize "YYYY-MM-DD HH:MM:SS" to ISO 8601 UTC so Date.parse works
// consistently across browsers and aligns with the generated buckets.
export function normalizeComputeTs(ts: string): string {
	return `${String(ts).replace(" ", "T")}Z`;
}

// Derive the per-bucket cost series from the raw active_seconds / cpu /
// memory_mib columns. Because the endpoint emits active-time-weighted cpu and
// memory, active_seconds * computeCostPerSecond(cpu, memory) per bucket equals
// the exact sum over individual instances. See @/content/billing.
export function parseComputeCostPoints(
	data: MetricsColumnar,
): { ts: string; value: number }[] {
	const byTs = new Map<
		string,
		{ activeSeconds: number; cpu: number; memoryMib: number }
	>();
	for (let i = 0; i < data.name.length; i++) {
		const ts = normalizeComputeTs(data.ts[i]);
		const value = Number(data.value[i]);
		const entry = byTs.get(ts) ?? {
			activeSeconds: 0,
			cpu: 0,
			memoryMib: 0,
		};
		if (data.name[i] === "active_seconds") entry.activeSeconds = value;
		else if (data.name[i] === "cpu") entry.cpu = value;
		else if (data.name[i] === "memory_mib") entry.memoryMib = value;
		byTs.set(ts, entry);
	}

	return Array.from(byTs.entries())
		.map(([ts, { activeSeconds, cpu, memoryMib }]) => ({
			ts,
			value: activeSeconds * computeCostPerSecond(cpu, memoryMib),
		}))
		.sort((a, b) => a.ts.localeCompare(b.ts));
}

// Total compute cost in dollars across all buckets returned by the compute
// metrics endpoint.
export function sumComputeCost(data: MetricsColumnar | undefined): number {
	if (!data) return 0;
	return parseComputeCostPoints(data).reduce((sum, p) => sum + p.value, 0);
}

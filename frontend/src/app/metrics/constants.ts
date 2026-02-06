import { bytes, formatBytes } from "@/utils/bytes";
import type { MetricConfig, MetricName, TimeRangeOption } from "./types";

export const ALL_METRICS: MetricName[] = [
	"actor_awake",
	"total_actors",
	"kv_storage_used",
	"kv_read",
	"kv_write",
	"gateway_ingress",
	"gateway_egress",
	"requests",
	"active_requests",
	"alarms_set",
];

function formatSeconds(value: number): string {
	const hours = value / 3600;
	if (hours >= 1000) return `${(hours / 1000).toFixed(1)}k hrs`;
	if (hours >= 1) return `${hours.toFixed(1)} hrs`;
	const minutes = value / 60;
	if (minutes >= 1) return `${minutes.toFixed(1)} min`;
	return `${value.toFixed(0)} sec`;
}

function formatCount(value: number): string {
	if (value >= 1_000_000) return `${(value / 1_000_000).toFixed(1)}M`;
	if (value >= 1_000) return `${(value / 1_000).toFixed(1)}K`;
	return value.toFixed(0);
}

function formatOperations(value: number): string {
	const units = value / bytes.KiB(4); // 4KB operation units
	if (units >= 1_000_000_000)
		return `${(units / 1_000_000_000).toFixed(2)}B ops`;
	if (units >= 1_000_000) return `${(units / 1_000_000).toFixed(2)}M ops`;
	if (units >= 1_000) return `${(units / 1_000).toFixed(2)}K ops`;
	return `${Math.round(units)} ops`;
}

export const METRICS_CONFIG: MetricConfig[] = [
	{
		name: "actor_awake",
		title: "Awake Actors",
		description: "Time actors spend running and processing requests",
		formatValue: formatSeconds,
	},
	{
		name: "total_actors",
		title: "Total Actors",
		description: "Number of active actors",
		formatValue: formatCount,
	},
	{
		name: "kv_storage_used",
		title: "KV Storage Used",
		description: "Persistent data stored in actor state",
		formatValue: formatBytes,
	},
	{
		name: "kv_read",
		title: "KV Reads",
		description: "Data read from actor state (4KiB units)",
		formatValue: formatOperations,
	},
	{
		name: "kv_write",
		title: "KV Writes",
		description: "Data written to actor state (4KiB units)",
		formatValue: formatOperations,
	},
	{
		name: "gateway_ingress",
		title: "Gateway Ingress",
		description: "Network traffic received from external clients",
		formatValue: formatBytes,
	},
	{
		name: "gateway_egress",
		title: "Gateway Egress",
		description: "Network traffic sent to external clients",
		formatValue: formatBytes,
	},
	{
		name: "requests",
		title: "Requests",
		description: "Total number of requests",
		formatValue: formatCount,
	},
	{
		name: "active_requests",
		title: "Active Requests",
		description: "Currently processing requests",
		formatValue: formatCount,
	},
	{
		name: "alarms_set",
		title: "Alarms Set",
		description: "Number of scheduled alarms",
		formatValue: formatCount,
	},
];

export const TIME_RANGE_OPTIONS: TimeRangeOption[] = [
	{
		label: "15m",
		value: "15m",
		milliseconds: 15 * 60 * 1000,
		resolution: 15,
	},
	{ label: "1h", value: "1h", milliseconds: 60 * 60 * 1000, resolution: 60 },
	{
		label: "6h",
		value: "6h",
		milliseconds: 6 * 60 * 60 * 1000,
		resolution: 300,
	},
	{
		label: "24h",
		value: "24h",
		milliseconds: 24 * 60 * 60 * 1000,
		resolution: 800,
	},
	{
		label: "7d",
		value: "7d",
		milliseconds: 7 * 24 * 60 * 60 * 1000,
		resolution: 800,
	},
	{
		label: "30d",
		value: "30d",
		milliseconds: 30 * 24 * 60 * 60 * 1000,
		resolution: 800,
	},
];

export const NAMESPACE_COLORS = [
	"hsl(var(--chart-1))",
	"hsl(var(--chart-2))",
	"hsl(var(--chart-3))",
	"hsl(var(--chart-4))",
	"hsl(var(--chart-5))",
	"hsl(220, 70%, 50%)",
	"hsl(160, 60%, 45%)",
	"hsl(280, 65%, 55%)",
	"hsl(45, 80%, 50%)",
	"hsl(0, 65%, 55%)",
];

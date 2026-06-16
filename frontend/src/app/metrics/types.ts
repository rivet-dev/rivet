import type { Rivet } from "@rivet-gg/cloud";

export type MetricName = Rivet.MetricName;

export interface MetricConfig {
	name: MetricName;
	title: string;
	description: string;
	formatValue: (value: number) => string;
}

// Raw compute usage metric names returned by the compute-metrics endpoint.
export type ComputeMetricName = "active_seconds" | "cpu" | "memory_mib";

// Compute panels also render a frontend-derived "cost" series, which is not a
// raw metric returned by the endpoint but computed from the others.
export type ComputeMetricKey = ComputeMetricName | "cost";

export interface ComputeMetricConfig {
	name: ComputeMetricKey;
	title: string;
	description: string;
	formatValue: (value: number) => string;
}

// Columnar metrics payload shared by the actor and compute metrics endpoints.
export interface MetricsColumnar {
	name: string[];
	ts: string[];
	value: string[];
}

export interface TimeRangeOption {
	label: string;
	value: string;
	milliseconds: number;
	resolution: number;
}

export interface MetricDataPoint {
	ts: string;
	value: number;
}

export type NamespaceMetricsData = Record<MetricName, MetricDataPoint[]>;

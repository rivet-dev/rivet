import type { Rivet } from "@rivet-gg/cloud";

export type MetricName = Rivet.MetricName;

export interface MetricConfig {
	name: MetricName;
	title: string;
	description: string;
	formatValue: (value: number) => string;
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

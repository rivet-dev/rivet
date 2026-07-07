import {
	faBarcodeRead,
	faDatabase,
	faPencil,
	faRunning,
	faSignalStream,
	type IconProp,
} from "@rivet-gg/icons";
import type { MetricType } from "@/app/billing/usage-card";

export interface UsageMetricConfig {
	key: string;
	title: string;
	description: string;
	icon: IconProp;
	metricType: MetricType;
}

// Display metadata for the metrics the backend returns from the billing usage
// endpoint, in render order. The endpoint is the source of truth for WHICH
// metrics exist and their numbers; this only supplies titles, descriptions, and
// icons. Compute metrics are billed on the backend but not yet returned by the
// usage endpoint, so they have no card here.
export const USAGE_METRICS: UsageMetricConfig[] = [
	{
		key: "actor_awake",
		title: "Awake actors",
		description: "Time your actors spend running and processing requests.",
		icon: faRunning,
		metricType: "hours",
	},
	{
		key: "kv_storage_used",
		title: "State storage",
		description:
			"Persistent data stored in actor state across all namespaces.",
		icon: faDatabase,
		metricType: "bytes",
	},
	{
		key: "kv_read",
		title: "Reads",
		description: "Data read from actor state, measured in 4KiB units.",
		icon: faBarcodeRead,
		metricType: "operations",
	},
	{
		key: "kv_write",
		title: "Writes",
		description: "Data written to actor state, measured in 4KiB units.",
		icon: faPencil,
		metricType: "operations",
	},
	{
		key: "gateway_egress",
		title: "Egress",
		description:
			"Network traffic sent from your actors to external clients.",
		icon: faSignalStream,
		metricType: "bytes",
	},
];

import { useQueries } from "@tanstack/react-query";
import { useMemo } from "react";
import { useCloudProjectDataProvider } from "@/components/actors";
import { ALL_METRICS } from "./constants";
import type { MetricName, NamespaceMetricsData } from "./types";

interface UseNamespaceMetricsOptions {
	namespaces: string[];
	startAt: string;
	endAt: string;
	resolution: number;
}

export function useNamespaceMetrics({
	namespaces,
	startAt,
	endAt,
	resolution,
}: UseNamespaceMetricsOptions) {
	const dataProvider = useCloudProjectDataProvider();

	const queries = useQueries({
		queries: namespaces.map((namespace) => ({
			...dataProvider.currentProjectNamespaceMetricsQueryOptions({
				namespace,
				name: ALL_METRICS,
				startAt,
				endAt,
				resolution,
			}),
			refetchInterval: 30000,
		})),
	});

	const data = useMemo(() => {
		const result = new Map<string, NamespaceMetricsData>();

		namespaces.forEach((namespace, index) => {
			const query = queries[index];
			if (!query.data) return;

			const nsData: NamespaceMetricsData = {} as NamespaceMetricsData;

			for (let i = 0; i < query.data.name.length; i++) {
				const metricName = query.data.name[i] as MetricName;
				const ts = query.data.ts[i];
				const value = Number(query.data.value[i]);

				if (!nsData[metricName]) {
					nsData[metricName] = [];
				}

				nsData[metricName].push({ ts, value });
			}

			result.set(namespace, nsData);
		});

		return result;
	}, [namespaces, queries]);

	const isLoading = queries.some((q) => q.isLoading);
	const isError = queries.some((q) => q.isError);

	return { data, isLoading, isError };
}

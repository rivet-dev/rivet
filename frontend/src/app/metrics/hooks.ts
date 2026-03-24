import { keepPreviousData, useQueries, useQuery } from "@tanstack/react-query";
import { useMemo } from "react";
import {
	useCloudNamespaceDataProvider,
	useCloudProjectDataProvider,
} from "@/components/actors";
import { ALL_METRICS } from "./constants";
import type { MetricName, NamespaceMetricsData } from "./types";

// 7 days overview range for the brush
export const OVERVIEW_RANGE_MS = 7 * 24 * 60 * 60 * 1000;
export const OVERVIEW_RESOLUTION = 800;

// 1 day default detail range for the main chart
export const DETAIL_RANGE_MS = 24 * 60 * 60 * 1000;
export const DETAIL_RESOLUTION = 60;

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
			placeholderData: keepPreviousData,
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
				// Normalize "YYYY-MM-DD HH:MM:SS" to ISO 8601 UTC so Date.parse works
				// consistently across all browsers and aligns with the generated buckets.
				const ts = `${String(query.data.ts[i]).replace(" ", "T")}Z`;
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

const VALID_RESOLUTIONS = [15, 60, 300, 800];

function detailRangeFromBrush(brushDomain: [Date, Date]): {
	startAt: string;
	endAt: string;
	resolution: number;
} {
	const rangeMs = brushDomain[1].getTime() - brushDomain[0].getTime();
	// Pick the smallest valid resolution that yields at most ~200 points.
	const ideal = rangeMs / (200 * 1000);
	const resolution = VALID_RESOLUTIONS.find((r) => r >= ideal) ?? VALID_RESOLUTIONS[VALID_RESOLUTIONS.length - 1];
	return {
		startAt: brushDomain[0].toISOString(),
		endAt: brushDomain[1].toISOString(),
		resolution,
	};
}

export function useNamespaceDetailMetrics({
	namespaces,
	brushDomain,
}: {
	namespaces: string[];
	brushDomain: [Date, Date];
}) {
	const { startAt, endAt, resolution } = useMemo(
		() => detailRangeFromBrush(brushDomain),
		[brushDomain],
	);

	return { ...useNamespaceMetrics({ namespaces, startAt, endAt, resolution }), startAt, endAt, resolution };
}

export function useSingleNamespaceDetailMetrics({
	brushDomain,
}: {
	brushDomain: [Date, Date];
}) {
	const dataProvider = useCloudNamespaceDataProvider();

	const { startAt, endAt, resolution } = useMemo(
		() => detailRangeFromBrush(brushDomain),
		[brushDomain],
	);

	const query = useQuery({
		...dataProvider.currentNamespaceMetricsQueryOptions({
			name: ALL_METRICS,
			startAt,
			endAt,
			resolution,
		}),
		refetchInterval: 30000,
		placeholderData: keepPreviousData,
	});

	return { query, startAt, endAt, resolution };
}

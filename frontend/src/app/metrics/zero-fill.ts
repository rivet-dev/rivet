import type { MetricDataPoint } from "./types";

interface ZeroFillOptions {
	startAt: string;
	endAt: string;
	resolution: number;
}

/**
 * Generate a complete time series by filling in missing timestamps with zero values.
 * The backend does not return 0s for empty time buckets, so this function
 * creates all expected timestamps from startAt to endAt, stepping by
 * resolution seconds, and inserts 0 for any missing ones.
 */
export function zeroFillDataPoints(
	dataPoints: MetricDataPoint[],
	options: ZeroFillOptions,
): MetricDataPoint[] {
	const { startAt, endAt, resolution } = options;
	const startMs = new Date(startAt).getTime();
	const endMs = new Date(endAt).getTime();
	const resolutionMs = resolution * 1000;

	// Build a lookup from timestamp (rounded to resolution boundary) to value.
	const valueMap = new Map<number, number>();
	for (const dp of dataPoints) {
		const tsMs = new Date(dp.ts).getTime();
		const bucket = Math.round(tsMs / resolutionMs) * resolutionMs;
		valueMap.set(bucket, (valueMap.get(bucket) ?? 0) + dp.value);
	}

	// Generate all time buckets from start to end.
	const result: MetricDataPoint[] = [];
	for (let t = startMs; t <= endMs; t += resolutionMs) {
		result.push({
			ts: new Date(t).toISOString(),
			value: valueMap.get(t) ?? 0,
		});
	}

	return result;
}

/**
 * Lightweight in-memory metrics for actor instances.
 *
 * Metrics are collected per actor wake cycle and are NOT persisted. They reset
 * when the actor sleeps and wakes again.
 */

export interface CounterMetric {
	type: "counter";
	help: string;
	value: number;
}

export interface GaugeMetric {
	type: "gauge";
	help: string;
	value: number;
}

export interface LabeledCounterMetric {
	type: "labeled_counter";
	help: string;
	values: Record<string, number>;
}

export interface LabeledTimingMetric {
	type: "labeled_timing";
	help: string;
	values: Record<string, { calls: number; totalMs: number; keys: number }>;
}

export type Metric =
	| CounterMetric
	| GaugeMetric
	| LabeledCounterMetric
	| LabeledTimingMetric;

export type MetricsSnapshot = Record<string, Metric>;

export class ActorMetrics {
	// KV operations
	kvGet = { calls: 0, keys: 0, totalMs: 0 };
	kvGetBatch = { calls: 0, keys: 0, totalMs: 0 };
	kvPut = { calls: 0, keys: 0, totalMs: 0 };
	kvPutBatch = { calls: 0, keys: 0, totalMs: 0 };
	kvDeleteBatch = { calls: 0, keys: 0, totalMs: 0 };

	// SQL statements
	sqlSelects = 0;
	sqlInserts = 0;
	sqlUpdates = 0;
	sqlDeletes = 0;
	sqlOther = 0;
	sqlTotalMs = 0;

	// Actions
	actionCalls = 0;
	actionErrors = 0;
	actionTotalMs = 0;

	// Connections
	connectionsOpened = 0;
	connectionsClosed = 0;

	trackSql(query: string, durationMs: number): void {
		const token = query.trimStart().slice(0, 8).toUpperCase();
		if (token.startsWith("SELECT") || token.startsWith("PRAGMA") || token.startsWith("WITH")) {
			this.sqlSelects++;
		} else if (token.startsWith("INSERT")) {
			this.sqlInserts++;
		} else if (token.startsWith("UPDATE")) {
			this.sqlUpdates++;
		} else if (token.startsWith("DELETE")) {
			this.sqlDeletes++;
		} else {
			this.sqlOther++;
		}
		this.sqlTotalMs += durationMs;
	}

	snapshot(): MetricsSnapshot {
		return {
			kv_operations: {
				type: "labeled_timing",
				help: "KV round trips by operation type",
				values: {
					get: { ...this.kvGet },
					getBatch: { ...this.kvGetBatch },
					put: { ...this.kvPut },
					putBatch: { ...this.kvPutBatch },
					deleteBatch: { ...this.kvDeleteBatch },
				},
			},
			sql_statements: {
				type: "labeled_counter",
				help: "SQL statements executed by type",
				values: {
					select: this.sqlSelects,
					insert: this.sqlInserts,
					update: this.sqlUpdates,
					delete: this.sqlDeletes,
					other: this.sqlOther,
				},
			},
			sql_duration_ms: {
				type: "counter",
				help: "Total SQL execution time in milliseconds",
				value: this.sqlTotalMs,
			},
			action_calls: {
				type: "counter",
				help: "Total action invocations",
				value: this.actionCalls,
			},
			action_errors: {
				type: "counter",
				help: "Total action errors",
				value: this.actionErrors,
			},
			action_duration_ms: {
				type: "counter",
				help: "Total action execution time in milliseconds",
				value: this.actionTotalMs,
			},
			connections_opened: {
				type: "counter",
				help: "Total WebSocket connections opened",
				value: this.connectionsOpened,
			},
			connections_closed: {
				type: "counter",
				help: "Total WebSocket connections closed",
				value: this.connectionsClosed,
			},
		};
	}
}

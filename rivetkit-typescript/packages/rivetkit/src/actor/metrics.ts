/**
 * Lightweight in-memory metrics for actor instances.
 *
 * Metrics are collected per actor wake cycle and are NOT persisted. They reset
 * when the actor sleeps and wakes again.
 */

export interface SqliteVfsMetricsSnapshot {
	requestBuildNs: number;
	serializeNs: number;
	transportNs: number;
	stateUpdateNs: number;
	totalNs: number;
	commitCount: number;
}

/** Keys of `ActorMetrics["startup"]` whose values are `number`. */
export type StartupTimingKey = {
	[K in keyof ActorMetrics["startup"]]: ActorMetrics["startup"][K] extends number
		? K
		: never;
}[keyof ActorMetrics["startup"]];

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
	#sqliteVfsMetricsSource?: () => SqliteVfsMetricsSnapshot | null;

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

	// Startup timing
	startup = {
		isNew: false,
		totalMs: 0,
		kvRoundTrips: 0,
		// Internal
		checkPersistDataMs: 0,
		initNewActorMs: 0,
		preloadKvMs: 0,
		preloadKvEntries: 0,
		instantiateMs: 0,
		loadStateMs: 0,
		restoreConnectionsMs: 0,
		restoreConnectionsCount: 0,
		initQueueMs: 0,
		initInspectorTokenMs: 0,
		flushWritesMs: 0,
		flushWritesEntries: 0,
		setupDatabaseClientMs: 0,
		initAlarmsMs: 0,
		onBeforeActorStartMs: 0,
		// User
		createStateMs: 0,
		onCreateMs: 0,
		onWakeMs: 0,
		createVarsMs: 0,
		dbMigrateMs: 0,
	};

	/** Total number of KV read calls made so far. */
	get totalKvReads(): number {
		return this.kvGet.calls + this.kvGetBatch.calls;
	}

	/** Total number of KV write calls made so far. */
	get totalKvWrites(): number {
		return (
			this.kvPut.calls + this.kvPutBatch.calls + this.kvDeleteBatch.calls
		);
	}

	trackSql(query: string, durationMs: number): void {
		const token = query.trimStart().slice(0, 8).toUpperCase();
		if (
			token.startsWith("SELECT") ||
			token.startsWith("PRAGMA") ||
			token.startsWith("WITH")
		) {
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

	setSqliteVfsMetricsSource(
		source?: () => SqliteVfsMetricsSnapshot | null,
	): void {
		this.#sqliteVfsMetricsSource = source;
	}

	snapshot(): MetricsSnapshot {
		const s = this.startup;
		const sqliteVfsMetrics = this.#sqliteVfsMetricsSource?.() ?? {
			requestBuildNs: 0,
			serializeNs: 0,
			transportNs: 0,
			stateUpdateNs: 0,
			totalNs: 0,
			commitCount: 0,
		};
		const commitCalls = sqliteVfsMetrics.commitCount;
		const nsToMs = (ns: number) => ns / 1_000_000;
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
			sqlite_commit_phases: {
				type: "labeled_timing",
				help: "SQLite VFS commit phase totals captured by the native VFS",
				values: {
					request_build: {
						calls: commitCalls,
						totalMs: nsToMs(sqliteVfsMetrics.requestBuildNs),
						keys: 0,
					},
					serialize: {
						calls: commitCalls,
						totalMs: nsToMs(sqliteVfsMetrics.serializeNs),
						keys: 0,
					},
					transport: {
						calls: commitCalls,
						totalMs: nsToMs(sqliteVfsMetrics.transportNs),
						keys: 0,
					},
					state_update: {
						calls: commitCalls,
						totalMs: nsToMs(sqliteVfsMetrics.stateUpdateNs),
						keys: 0,
					},
				},
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
			startup_total_ms: {
				type: "gauge",
				help: "Total actor startup time in milliseconds",
				value: s.totalMs,
			},
			startup_kv_round_trips: {
				type: "gauge",
				help: "KV round-trips during startup",
				value: s.kvRoundTrips,
			},
			startup_is_new: {
				type: "gauge",
				help: "1 if new actor, 0 if existing",
				value: s.isNew ? 1 : 0,
			},
			startup_internal_check_persist_data_ms: {
				type: "gauge",
				help: "Time to check persist data existence",
				value: s.checkPersistDataMs,
			},
			startup_internal_init_new_actor_ms: {
				type: "gauge",
				help: "Time to write initial KV state for new actor",
				value: s.initNewActorMs,
			},
			startup_internal_preload_kv_ms: {
				type: "gauge",
				help: "Time to preload startup KV data",
				value: s.preloadKvMs,
			},
			startup_internal_preload_kv_entries: {
				type: "gauge",
				help: "Number of entries preloaded",
				value: s.preloadKvEntries,
			},
			startup_internal_instantiate_ms: {
				type: "gauge",
				help: "Time to instantiate actor class",
				value: s.instantiateMs,
			},
			startup_internal_load_state_ms: {
				type: "gauge",
				help: "Time to load and deserialize actor state",
				value: s.loadStateMs,
			},
			startup_internal_restore_connections_ms: {
				type: "gauge",
				help: "Time to restore persisted connections",
				value: s.restoreConnectionsMs,
			},
			startup_internal_restore_connections_count: {
				type: "gauge",
				help: "Number of connections restored",
				value: s.restoreConnectionsCount,
			},
			startup_internal_init_queue_ms: {
				type: "gauge",
				help: "Time to initialize queue metadata",
				value: s.initQueueMs,
			},
			startup_internal_init_inspector_token_ms: {
				type: "gauge",
				help: "Time to load or generate inspector token",
				value: s.initInspectorTokenMs,
			},
			startup_internal_flush_writes_ms: {
				type: "gauge",
				help: "Time to flush batched init writes",
				value: s.flushWritesMs,
			},
			startup_internal_flush_writes_entries: {
				type: "gauge",
				help: "Number of entries in batched init write",
				value: s.flushWritesEntries,
			},
			startup_internal_setup_database_client_ms: {
				type: "gauge",
				help: "Time to create database client",
				value: s.setupDatabaseClientMs,
			},
			startup_internal_init_alarms_ms: {
				type: "gauge",
				help: "Time to initialize scheduled alarms",
				value: s.initAlarmsMs,
			},
			startup_internal_on_before_actor_start_ms: {
				type: "gauge",
				help: "Time for driver onBeforeActorStart hook",
				value: s.onBeforeActorStartMs,
			},
			startup_user_create_state_ms: {
				type: "gauge",
				help: "Time in user createState callback",
				value: s.createStateMs,
			},
			startup_user_on_create_ms: {
				type: "gauge",
				help: "Time in user onCreate callback",
				value: s.onCreateMs,
			},
			startup_user_on_wake_ms: {
				type: "gauge",
				help: "Time in user onWake callback",
				value: s.onWakeMs,
			},
			startup_user_create_vars_ms: {
				type: "gauge",
				help: "Time in user createVars callback",
				value: s.createVarsMs,
			},
			startup_user_db_migrate_ms: {
				type: "gauge",
				help: "Time in user database migration",
				value: s.dbMigrateMs,
			},
		};
	}
}

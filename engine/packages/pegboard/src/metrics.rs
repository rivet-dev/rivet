use rivet_metrics::{BUCKETS, MICRO_BUCKETS, REGISTRY, START_BUCKETS, prometheus::*};

lazy_static::lazy_static! {
	pub static ref NAMESPACE_GROUP_CREATES: IntCounterVec = register_int_counter_vec_with_registry!(
		"pegboard_namespace_group_creates_total",
		"Root namespace group creates that finished, by result: ok or unknown.",
		&["result"],
		*REGISTRY
	).unwrap();
	pub static ref NAMESPACE_FORKS: IntCounterVec = register_int_counter_vec_with_registry!(
		"pegboard_namespace_forks_total",
		"Namespace forks whose group writes finished, by result: ok, refused or unknown.",
		&["result"],
		*REGISTRY
	).unwrap();
	pub static ref NAMESPACE_ARCHIVES: IntCounterVec = register_int_counter_vec_with_registry!(
		"pegboard_namespace_archives_total",
		"Namespace archives whose group archive finished, by result: ok, refused or unknown.",
		&["result"],
		*REGISTRY
	).unwrap();
	pub static ref ACTOR_EFFECT_REFUSALS: IntCounterVec = register_int_counter_vec_with_registry!(
		"pegboard_actor_effect_refusals_total",
		"Actor effect attempts depot refused at admission, which the actor state machine runs again, by effect and refusal reason.",
		&["effect", "reason"],
		*REGISTRY
	).unwrap();
	pub static ref ACTOR_EVENT_REPORT_REFUSALS: IntCounterVec = register_int_counter_vec_with_registry!(
		"pegboard_actor_event_report_refusals_total",
		"Effect results the actor state volume refused at admission and the executor sent again, by event and refusal reason.",
		&["event", "reason"],
		*REGISTRY
	).unwrap();
	pub static ref ACTOR_ALARM_RETRIES: IntCounterVec = register_int_counter_vec_with_registry!(
		"pegboard_actor_alarm_retries_total",
		"Actor alarms armed again after a start that did not complete, so the alarm is not lost, by reason: start_timeout or start_failed.",
		&["reason"],
		*REGISTRY
	).unwrap();
	pub static ref ACTOR_FORK_RESETS: IntCounterVec = register_int_counter_vec_with_registry!(
		"pegboard_actor_fork_resets_total",
		"Inherited actor states reset for a child namespace's volume, by the phase they inherited.",
		&["phase"],
		*REGISTRY
	).unwrap();
	pub static ref ACTOR_FORK_STEPS: IntCounterVec = register_int_counter_vec_with_registry!(
		"pegboard_actor_fork_steps_total",
		"Reported steps of individual actor database forks, by namespace and result: prepared, forked, failed or refused.",
		&["namespace_id", "result"],
		*REGISTRY
	).unwrap();
	pub static ref ACTOR_VALUES_SEALED: IntCounterVec = register_int_counter_vec_with_registry!(
		"pegboard_actor_values_sealed_total",
		"pegboard.actor values that a process other than the writer reads, sealed at the version their readers read, by value (kv, reservation or start_notice) and version. A count at an older version shows the fleet still runs a release that reads only that version.",
		&["value", "version"],
		*REGISTRY
	).unwrap();
	pub static ref ENVOY_INTERNAL_DOWNGRADES: IntCounterVec = register_int_counter_vec_with_registry!(
		"pegboard_envoy_internal_downgrades_total",
		"Engine-internal envoy messages encoded below the latest envoy protocol version because the receiving engine worker reads an older one or topology does not know it, by message: start, stop, tunnel or tunnel_reply. Tunnels count once each.",
		&["message"],
		*REGISTRY
	).unwrap();
	pub static ref ACTOR_FORKS: IntCounterVec = register_int_counter_vec_with_registry!(
		"pegboard_actor_forks_total",
		"Individual actor fork requests, by namespace and result: ok, duplicate_key, too_deep, failed, pending, unavailable, moving, not_found or error.",
		&["namespace_id", "result"],
		*REGISTRY
	).unwrap();
	pub static ref ACTOR_FORK_SECONDS: HistogramVec = register_histogram_vec_with_registry!(
		"pegboard_actor_fork_seconds",
		"Time from a forked actor's create to its database fork finishing, by namespace, measured by the actor state machine's executor.",
		&["namespace_id"],
		BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();
	pub static ref NAMESPACE_RETRIES: IntCounterVec = register_int_counter_vec_with_registry!(
		"pegboard_namespace_retries_total",
		"Re-sends of a namespace state machine's effects by its retry timers, by kind: fork, archive or index.",
		&["kind"],
		*REGISTRY
	).unwrap();
	pub static ref NAMESPACE_START_WAITERS_DROPPED: IntCounter = register_int_counter_with_registry!(
		"pegboard_namespace_start_waiters_dropped_total",
		"Namespace creates whose start waiter was left out because the namespace's waiter list was full; they read the namespace at their deadline.",
		*REGISTRY
	).unwrap();
	pub static ref NAMESPACE_STATUS_READS: IntCounter = register_int_counter_with_registry!(
		"pegboard_namespace_status_reads_total",
		"Reads of a namespace's state from its volume, made on a cache miss or a stale status.",
		*REGISTRY
	).unwrap();
	pub static ref NAMESPACE_STATUS_REFRESHES: IntGauge = register_int_gauge_with_registry!(
		"pegboard_namespace_status_refreshes",
		"Background reads of a stale namespace status this process runs now.",
		*REGISTRY
	).unwrap();
	pub static ref NAMESPACE_STATUS_REFRESHES_CAPACITY: IntGauge = register_int_gauge_with_registry!(
		"pegboard_namespace_status_refreshes_capacity",
		"The configured capacity of pegboard_namespace_status_refreshes.",
		*REGISTRY
	).unwrap();
	pub static ref NAMESPACE_STATUS_REFRESHES_SKIPPED: IntCounter = register_int_counter_with_registry!(
		"pegboard_namespace_status_refreshes_skipped_total",
		"Stale namespace statuses answered without starting a background read because every slot was taken.",
		*REGISTRY
	).unwrap();
	/// Legacy: removed in 3.1 with the backfill.
	pub static ref BACKFILL_ACTORS_MOVED: IntCounterVec = register_int_counter_vec_with_registry!(
		"pegboard_backfill_actors_moved_total",
		"Actors from before volume groups that a namespace's backfill moved into its group, by namespace.",
		&["namespace_id"],
		*REGISTRY
	).unwrap();
	/// Legacy: removed in 3.1 with the backfill.
	pub static ref BACKFILL_PAGES: IntCounterVec = register_int_counter_vec_with_registry!(
		"pegboard_backfill_pages_total",
		"Backfill pages run, by namespace and result: next (more pages follow), finished (the walk ended) or failed (nothing moved; the page is sent again).",
		&["namespace_id", "result"],
		*REGISTRY
	).unwrap();
	/// Legacy: counted by the namespace state machine's executor for each accepted transition, so
	/// the namespaces in a phase are its entries less the next phase's. Removed in 3.1 with the
	/// backfill.
	pub static ref BACKFILL_PHASE_ENTRIES: IntCounterVec = register_int_counter_vec_with_registry!(
		"pegboard_backfill_phase_entries_total",
		"Namespace backfills entering a phase: pending (moved from a legacy row), running or done.",
		&["phase"],
		*REGISTRY
	).unwrap();
	/// Legacy: counted once per skip by the namespace state machine's accepted transition.
	/// Removed in 3.1 with the backfill.
	pub static ref BACKFILL_SKIPPED: IntCounterVec = register_int_counter_vec_with_registry!(
		"pegboard_backfill_skipped_total",
		"Actors from before volume groups a namespace's backfill gave up on after its configured attempts and left in the legacy shape, by namespace. 3.1 refuses to start while any are left.",
		&["namespace_id"],
		*REGISTRY
	).unwrap();
	/// Legacy: set from each skip and each refused give-up. Removed in 3.1 with the backfill.
	pub static ref BACKFILL_SKIPPED_LISTED: IntGaugeVec = register_int_gauge_vec_with_registry!(
		"pegboard_backfill_skipped_listed",
		"Actors a namespace's running backfill gave up on and carries in its state, by namespace.",
		&["namespace_id"],
		*REGISTRY
	).unwrap();
	/// Legacy: removed in 3.1 with the backfill.
	pub static ref BACKFILL_SKIPPED_CAPACITY: IntGauge = register_int_gauge_with_registry!(
		"pegboard_backfill_skipped_capacity",
		"The configured capacity of pegboard_backfill_skipped_listed.",
		*REGISTRY
	).unwrap();
	/// Legacy: counted once per page send that reached its attempts at the cap. Removed in 3.1
	/// with the backfill.
	pub static ref BACKFILL_GIVE_UPS_REFUSED: IntCounterVec = register_int_counter_vec_with_registry!(
		"pegboard_backfill_give_ups_refused_total",
		"Page sends of a namespace's backfill that reached their attempts at an actor but did not give up on it, because the skipped actors were at their configured capacity, by namespace.",
		&["namespace_id"],
		*REGISTRY
	).unwrap();
	/// Legacy: removed in 3.1 with the backfill.
	pub static ref BACKFILL_RECORDED_NAMES: IntGauge = register_int_gauge_with_registry!(
		"pegboard_backfill_recorded_names",
		"Namespace and actor name pairs this process remembers having recorded for the backfill after a legacy fallback read.",
		*REGISTRY
	).unwrap();
	/// Legacy: removed in 3.1 with the backfill.
	pub static ref BACKFILL_RECORDED_NAMES_CAPACITY: IntGauge = register_int_gauge_with_registry!(
		"pegboard_backfill_recorded_names_capacity",
		"The configured capacity of pegboard_backfill_recorded_names.",
		*REGISTRY
	).unwrap();
	/// Legacy: removed in 3.1 with the backfill.
	pub static ref BACKFILL_NAME_RECORDS_SKIPPED: IntCounterVec = register_int_counter_vec_with_registry!(
		"pegboard_backfill_name_records_skipped_total",
		"Actor names a legacy fallback read did not record for the backfill, because this process's recorded names or name record writes in flight were at their configured capacity, by namespace. The backfill finds such a name only in the namespace's node records.",
		&["namespace_id"],
		*REGISTRY
	).unwrap();
	/// Legacy: held by each background name record write. Removed in 3.1 with the backfill.
	pub static ref BACKFILL_NAME_RECORD_WRITES: IntGauge = register_int_gauge_with_registry!(
		"pegboard_backfill_name_record_writes",
		"Backfill name record writes this process runs in the background.",
		*REGISTRY
	).unwrap();
	/// Legacy: removed in 3.1 with the backfill.
	pub static ref BACKFILL_NAME_RECORD_WRITES_CAPACITY: IntGauge = register_int_gauge_with_registry!(
		"pegboard_backfill_name_record_writes_capacity",
		"The configured capacity of pegboard_backfill_name_record_writes.",
		*REGISTRY
	).unwrap();
	/// Legacy: removed in 3.1 with the backfill.
	pub static ref BACKFILL_DUPLICATES: IntCounterVec = register_int_counter_vec_with_registry!(
		"pegboard_backfill_duplicates_total",
		"Legacy keyed actors a backfill destroyed instead of moving, because a live grouped actor already held their new id, by namespace.",
		&["namespace_id"],
		*REGISTRY
	).unwrap();
	pub static ref LEGACY_NAMESPACE_LIST_REFUSALS: IntCounter = register_int_counter_with_registry!(
		"pegboard_legacy_namespace_list_refusals_total",
		"Legacy namespace listings refused because namespaces already moved into their volumes while a storage worker of the previous release was listed again.",
		*REGISTRY
	).unwrap();
	pub static ref NAMESPACE_CREATING_SETTLES: IntCounterVec = register_int_counter_vec_with_registry!(
		"pegboard_namespace_creating_settles_total",
		"API requests that found their namespace creating, by result: settled (this request finished its group create), refused (another settle of the name ran or one failed moments ago) or failed (the settle, or a lookup of a namespace last read creating, failed or passed the request's deadline), which answer the retryable namespace.creating; and lookup_timed_out, a lookup of a namespace not known to be creating that passed the request's deadline, answered as depot.overloaded.",
		&["result"],
		*REGISTRY
	).unwrap();
	pub static ref NAMESPACE_LEGACY_MOVES: IntCounter = register_int_counter_with_registry!(
		"pegboard_namespace_legacy_moves_total",
		"Namespaces moved from their legacy catalog row into their namespace volume.",
		*REGISTRY
	).unwrap();
	/// Legacy: removed in 3.1 with the backfill.
	pub static ref LEGACY_NAMESPACE_ROW_PASS_MOVES: IntCounter = register_int_counter_with_registry!(
		"pegboard_legacy_namespace_row_pass_moves_total",
		"Legacy catalog rows the legacy namespace row pass moved into their namespace volume.",
		*REGISTRY
	).unwrap();
	/// Legacy: removed in 3.1 with the backfill.
	pub static ref LEGACY_NAMESPACE_ROW_PASSES_FINISHED: IntCounter = register_int_counter_with_registry!(
		"pegboard_legacy_namespace_row_passes_finished_total",
		"Legacy namespace row passes that ended with a full walk finding no legacy catalog row.",
		*REGISTRY
	).unwrap();
	/// Legacy: removed in 3.1 with the backfill.
	pub static ref LEGACY_NAMESPACE_ROW_PASS_REFUSALS: IntCounter = register_int_counter_with_registry!(
		"pegboard_legacy_namespace_row_pass_refusals_total",
		"Pages or moves of the legacy namespace row pass that failed or passed their deadline; the pass sends them again after its retry wait.",
		*REGISTRY
	).unwrap();
	/// Counted when the actor state machine commits the transition, so a retried command can
	/// count one stop twice.
	pub static ref ACTOR_STOPS: IntCounterVec = register_int_counter_vec_with_registry!(
		"pegboard_actor_stops_total",
		"Starting or running actor generations that began to stop, by kind: clean_exit, crash, lost, stop_intent, sleep_intent or engine_request.",
		&["kind"],
		*REGISTRY
	).unwrap();

	pub static ref ACTOR_START_WAITERS: IntCounterVec = register_int_counter_vec_with_registry!(
		"pegboard_actor_start_waiters_total",
		"Actor start waiter registrations and prunes, by result: stored, replaced, already_running, full or pruned.",
		&["result"],
		*REGISTRY
	).unwrap();

	pub static ref ACTOR_START_NOTICE_SEND_SECONDS: HistogramVec = register_histogram_vec_with_registry!(
		HistogramOpts::new("pegboard_actor_start_notice_send_seconds", "Time from a committed start-path transition to the waiting gateway worker's reply to its start notice, by result: sent, failed or timeout.").buckets(START_BUCKETS.to_vec()),
		&["result"],
		*REGISTRY
	).unwrap();

	pub static ref ACTOR_START_NOTICES: IntCounterVec = register_int_counter_vec_with_registry!(
		"pegboard_actor_start_notices_total",
		"Actor start notices sent to waiting gateway workers, by result: sent, failed or timeout.",
		&["result"],
		*REGISTRY
	).unwrap();

	pub static ref ACTOR_ALARMS: IntCounterVec = register_int_counter_vec_with_registry!(
		"pegboard_actor_alarms_total",
		"Actor alarm transitions, by result: set, cleared, fired_at_stop, fired_on_event or pending_unwired.",
		&["result"],
		*REGISTRY
	).unwrap();

	pub static ref ACTOR_EFFECT_GAVE_UP: IntCounterVec = register_int_counter_vec_with_registry!(
		"pegboard_actor_effect_gave_up_total",
		"Actor effects whose executor reached the attempt deadline while the state machine still waited on them.",
		&["effect"],
		*REGISTRY
	).unwrap();

	/// `trigger` is `gave_up`, `event` (an event found the effect overdue) or `read_check`.
	pub static ref ACTOR_EFFECT_RECOVERED: IntCounterVec = register_int_counter_vec_with_registry!(
		"pegboard_actor_effect_recovered_total",
		"Actor effects the state machine issued again for the same generation.",
		&["effect", "trigger"],
		*REGISTRY
	).unwrap();

	pub static ref ACTOR_EFFECT_RETRIES: IntCounterVec = register_int_counter_vec_with_registry!(
		"pegboard_actor_effect_retries_total",
		"Actor state machine effect attempts that failed and were retried.",
		&["effect"],
		*REGISTRY
	).unwrap();

	/// Creation to the first allocation decision, measured from the actor's data record.
	pub static ref ACTOR_CREATE_TO_ALLOCATED: HistogramVec = register_histogram_vec_with_registry!(
		HistogramOpts::new("pegboard_actor_create_to_allocated_seconds", "Time from actor creation to the allocation result of its first generation.").buckets(BUCKETS.to_vec()),
		&["result"],
		*REGISTRY
	).unwrap();

	/// One merged actor listing page across every index bucket.
	pub static ref ACTOR_INDEX_LIST_DURATION: Histogram = register_histogram_with_registry!(
		"pegboard_actor_index_list_duration_seconds",
		"Duration of one merged actor index listing page across every bucket.",
		BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();

	pub static ref ACTOR_INDEX_LIST_REFILLS: IntCounter = register_int_counter_with_registry!(
		"pegboard_actor_index_list_refills_total",
		"Extra bucket pages an actor listing read because one bucket held more rows of the page than its first read.",
		*REGISTRY
	).unwrap();

	pub static ref ENVOY_VERSION_EVICTIONS: IntCounterVec = register_int_counter_vec_with_registry!(
		"pegboard_envoy_version_evictions_total",
		"Actors asked to move off an envoy superseded by a newer version in a pool that drains on version upgrade, by namespace and pool.",
		&["namespace_id", "pool_name"],
		*REGISTRY
	).unwrap();

	/// Committed envoy index mutations, counted once per command rather than per actor name.
	pub static ref ENVOY_INDEX_UPSERTS: IntCounterVec = register_int_counter_vec_with_registry!(
		"pegboard_envoy_index_upserts_total",
		"Committed envoy index mutations by namespace and reason: connect, heartbeat or disconnect.",
		&["namespace_id", "reason"],
		*REGISTRY
	).unwrap();

	pub static ref ACTOR_VOLUME_DELETES: IntCounterVec = register_int_counter_vec_with_registry!(
		"pegboard_actor_volume_deletes_total",
		"Volume deletes of destroyed actors, by namespace, volume (state or database) and result: deleted, missing or error.",
		&["namespace", "volume", "result"],
		*REGISTRY
	).unwrap();

	pub static ref ENVOY_INDEX_STALE_ROWS: IntCounterVec = register_int_counter_vec_with_registry!(
		"pegboard_envoy_index_stale_rows_total",
		"Stale envoy index entries skipped by readers, by namespace, reason (missing_record or heartbeat_expired) and result (skipped). Repeated reads count repeated observations.",
		&["namespace_id", "reason", "result"],
		*REGISTRY
	).unwrap();

	pub static ref ACTOR_INDEX_LIST_BUCKETS: Histogram = register_histogram_with_registry!(
		"pegboard_actor_index_list_buckets",
		"Index bucket volumes one actor listing reads.",
		rivet_metrics::PAGE_COUNT_BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();

	pub static ref ACTOR_PENDING_ALLOCATION: IntGaugeVec = register_int_gauge_vec_with_registry!(
		"pegboard_actor_pending_allocation",
		"Total actors waiting for availability.",
		&["namespace", "runner_name"],
		*REGISTRY
	).unwrap();

	pub static ref ACTOR_ACTIVE: IntGaugeVec = register_int_gauge_vec_with_registry!(
		"pegboard_actor_active",
		"Total actors currently allocated.",
		&["namespace", "runner_name"],
		*REGISTRY
	).unwrap();

	pub static ref ACTOR_ALLOCATE_DURATION_SECONDS: HistogramVec = register_histogram_vec_with_registry!(
		"pegboard_actor_allocate_duration_seconds",
		"Total duration to allocate an actor.",
		&["kind", "result"],
		MICRO_BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();

	pub static ref ACTOR_START_DURATION_SECONDS: HistogramVec = register_histogram_vec_with_registry!(
		"pegboard_actor_start_duration_seconds",
		"Total duration from allocation to running.",
		&["namespace", "pool_name", "kind"],
		BUCKETS.to_vec(),
		*REGISTRY
	).unwrap();

	/// Storage latency at both sides of actor command routing.
	pub static ref STORAGE_OPERATION_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		HistogramOpts::new("pegboard_storage_operation_duration_seconds", "Actor storage duration at the partition or Envoy socket, including routing and lock waits at the socket.").buckets(MICRO_BUCKETS.to_vec()),
		&["operation", "boundary"],
		*REGISTRY
	).unwrap();

	/// Disjoint storage phases, including waits before the storage handler begins.
	pub static ref STORAGE_PHASE_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		HistogramOpts::new("pegboard_storage_phase_duration_seconds", "Storage command phases, including time spent waiting for partition ownership and durable WAL completion.").buckets(BUCKETS.to_vec()),
		&["phase"],
		*REGISTRY
	).unwrap();

	/// Wall-clock startup dependencies; parallel phase durations must not be summed.
	pub static ref STARTUP_PHASE_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		HistogramOpts::new("pegboard_startup_phase_duration_seconds", "Actor start phase duration, from creation dependencies through the first database open; parallel phases overlap and must not be summed.").buckets(MICRO_BUCKETS.to_vec()),
		&["phase"],
		*REGISTRY
	).unwrap();

	/// One actor event command, from send until its transition committed. `event` is
	/// `actor_machine::event_command_label`.
	pub static ref ACTOR_EVENT_COMMAND_SECONDS: HistogramVec = register_histogram_vec_with_registry!(
		HistogramOpts::new("pegboard_actor_event_command_seconds", "Actor event command latency by event, from send until its transition committed, including retries.").buckets(START_BUCKETS.to_vec()),
		&["event"],
		*REGISTRY
	).unwrap();

}

lazy_static::lazy_static! {
	/// Legacy: counted by the fallback resolves while a namespace's backfill is not done.
	/// Removed in 3.1 with the backfill.
	pub static ref LEGACY_ACTOR_READS: IntCounterVec = register_int_counter_vec_with_registry!(
		"pegboard_legacy_actor_reads_total",
		"Actor resolves through the legacy unprefixed fallback, by result: grouped (found in the namespace's group), legacy (found unprefixed) or not_found.",
		&["result"],
		*REGISTRY
	).unwrap();

	pub static ref ACTOR_KEY_ADOPTIONS: IntCounterVec = register_int_counter_vec_with_registry!(
		"pegboard_actor_key_adoptions_total",
		"Keys found reserved for an actor without state, which a getOrCreate adopted by creating that actor id, by result: adopted or error.",
		&["result"],
		*REGISTRY
	).unwrap();

	/// Recorded for a first start that had a waiting gateway, when its start notice is sent.
	pub static ref ACTOR_CREATE_TO_RUNNING_SECONDS: Histogram = register_histogram_with_registry!(
		HistogramOpts::new("pegboard_actor_create_to_running_seconds", "Time from an actor's creating transition to its first Running commit, for creates a gateway waited on.").buckets(START_BUCKETS.to_vec()),
		*REGISTRY
	).unwrap();

	/// Recorded for every start that had a waiting gateway, when its start notice is sent.
	pub static ref ACTOR_STARTING_TO_RUNNING_SECONDS: Histogram = register_histogram_with_registry!(
		HistogramOpts::new("pegboard_actor_starting_to_running_seconds", "Time from StartActor being emitted to the Running commit, for starts a gateway waited on.").buckets(START_BUCKETS.to_vec()),
		*REGISTRY
	).unwrap();
}

lazy_static::lazy_static! {
	/// Serial steps of the create and reallocate (wake) commands, so a slow start shows which
	/// step it waited on.
	pub static ref ACTOR_COMMAND_PHASE_SECONDS: HistogramVec = register_histogram_vec_with_registry!(
		HistogramOpts::new("pegboard_actor_command_phase_seconds", "Steps of the actor create and reallocate commands by command (create or reallocate) and phase: allocate (choosing the Envoy from memory), read_state (reading the actor's state before the event), commit (processing the event to its durable commit, with the key reservation overlapped on a create).").buckets(MICRO_BUCKETS.to_vec()),
		&["command", "phase"],
		*REGISTRY
	).unwrap();
}

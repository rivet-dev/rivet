use rivet_metrics::{BUCKETS, MICRO_BUCKETS, REGISTRY, START_BUCKETS, prometheus::*};

lazy_static::lazy_static! {
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

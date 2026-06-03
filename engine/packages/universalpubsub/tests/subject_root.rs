use universalpubsub::{InboxSubject, Subject, subject::subject_root_from_str};

#[test]
fn subject_root_prefix_table_matches_known_subjects() {
	let cases = [
		("pegboard.runner.abc", "pegboard.runner"),
		(
			"pegboard.runner.eviction-by-id.abc",
			"pegboard.runner.eviction-by-id",
		),
		(
			"pegboard.runner.eviction-by-name.ns.runner.key",
			"pegboard.runner.eviction-by-name",
		),
		("pegboard.gateway.abc", "pegboard.gateway"),
		("pegboard.envoy.ns.key", "pegboard.envoy"),
		("pegboard.envoy.eviction.ns.key", "pegboard.envoy.eviction"),
		(
			"pegboard.serverless.outbound",
			"pegboard.serverless.outbound",
		),
		("gasoline.worker.bump", "gasoline.worker.bump"),
		(
			"gasoline.workflow.created.746167",
			"gasoline.workflow.created",
		),
		(
			"gasoline.workflow.complete.abc",
			"gasoline.workflow.complete",
		),
		(
			"gasoline.signal.for-workflow.abc",
			"gasoline.signal.for-workflow",
		),
		("gasoline.msg.pegboard_actor_ready:global", "gasoline.msg"),
		("rivet.cache.purge", "rivet.cache.purge"),
		("rivet.debug.tracing.config", "rivet.debug.tracing.config"),
		("_INBOX.abc", "_inbox"),
		("other.subject", "unknown"),
	];

	for (subject, root) in cases {
		assert_eq!(subject_root_from_str(subject), root);
	}
}

#[test]
fn raw_subjects_do_not_infer_metric_roots() {
	assert_eq!("pegboard.envoy.ns.key".subject_root(), None);
}

#[test]
fn inbox_subject_reports_bounded_metric_root() {
	let subject = InboxSubject::new();
	assert_eq!(subject.subject_root().as_deref(), Some("_inbox"));
}

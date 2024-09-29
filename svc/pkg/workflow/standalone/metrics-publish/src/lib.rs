use chirp_workflow::prelude::*;

#[tracing::instrument(skip_all)]
pub async fn run_from_env(pools: rivet_pools::Pools) -> GlobalResult<()> {
	let client =
		chirp_client::SharedClient::from_env(pools.clone())?.wrap_new("workflow-metrics-publish");
	let cache = rivet_cache::CacheInner::from_env(pools.clone())?;
	let ctx = StandaloneCtx::new(
		chirp_workflow::compat::db_from_pools(&pools).await?,
		rivet_connection::Connection::new(client, pools, cache),
		"workflow-metrics-publish",
	)
	.await?;

	let (
		(active_worker_count,),
		total_workflow_count,
		active_workflow_count,
		dead_workflow_count,
		sleeping_workflow_count,
		pending_signal_count,
	) = tokio::try_join!(
		sql_fetch_one!(
			[ctx, (i64,)]
			"
			SELECT COUNT(*)
			FROM db_workflow.worker_instances AS OF SYSTEM TIME '-1s'
			WHERE last_ping_ts > $1
			",
			util::timestamp::now() - util::duration::seconds(30),
		),
		sql_fetch_all!(
			[ctx, (String, i64)]
			"
			SELECT workflow_name, COUNT(*)
			FROM db_workflow.workflows AS OF SYSTEM TIME '-1s'
			GROUP BY workflow_name
			",
		),
		sql_fetch_all!(
			[ctx, (String, i64)]
			"
			SELECT workflow_name, COUNT(*)
			FROM db_workflow.workflows AS OF SYSTEM TIME '-1s'
			WHERE
				output IS NULL AND
				worker_instance_id IS NOT NULL AND
				silence_ts IS NULL
			GROUP BY workflow_name
			",
		),
		sql_fetch_all!(
			[ctx, (String, String, i64)]
			"
			SELECT workflow_name, error, COUNT(*)
			FROM db_workflow.workflows AS OF SYSTEM TIME '-1s'
			WHERE
				error IS NOT NULL AND
				output IS NULL AND
				silence_ts IS NULL AND
				wake_immediate = FALSE AND
				wake_deadline_ts IS NULL AND
				cardinality(wake_signals) = 0 AND
				wake_sub_workflow_id IS NULL
			GROUP BY workflow_name, error
			",
		),
		sql_fetch_all!(
			[ctx, (String, i64)]
			"
			SELECT workflow_name, COUNT(*)
			FROM db_workflow.workflows AS OF SYSTEM TIME '-1s'
			WHERE
				worker_instance_id IS NULL AND
				output IS NULL AND
				silence_ts IS NULL AND
				(
					wake_immediate OR
					wake_deadline_ts IS NOT NULL OR
					cardinality(wake_signals) > 0 OR
					wake_sub_workflow_id IS NOT NULL
				)
			GROUP BY workflow_name
			",
		),
		sql_fetch_all!(
			[ctx, (String, i64)]
			"
			SELECT signal_name, COUNT(*)
			FROM (
				SELECT signal_name
				FROM db_workflow.signals
				WHERE
					ack_ts IS NULL AND
					silence_ts IS NULL
				UNION ALL
				SELECT signal_name
				FROM db_workflow.tagged_signals
				WHERE
					ack_ts IS NULL AND
					silence_ts IS NULL
			) AS OF SYSTEM TIME '-1s'
			GROUP BY signal_name
			",
		),
	)?;

	// Get rid of metrics that don't exist in the db anymore (declarative)
	chirp_workflow::metrics::WORKFLOW_TOTAL.reset();
	chirp_workflow::metrics::WORKFLOW_ACTIVE.reset();
	chirp_workflow::metrics::WORKFLOW_DEAD.reset();
	chirp_workflow::metrics::WORKFLOW_SLEEPING.reset();
	chirp_workflow::metrics::SIGNAL_PENDING.reset();

	chirp_workflow::metrics::WORKER_ACTIVE
		.with_label_values(&[])
		.set(active_worker_count);

	for (workflow_name, count) in total_workflow_count {
		chirp_workflow::metrics::WORKFLOW_TOTAL
			.with_label_values(&[&workflow_name])
			.set(count);
	}

	for (workflow_name, count) in active_workflow_count {
		chirp_workflow::metrics::WORKFLOW_ACTIVE
			.with_label_values(&[&workflow_name])
			.set(count);
	}

	for (workflow_name, error, count) in dead_workflow_count {
		chirp_workflow::metrics::WORKFLOW_DEAD
			.with_label_values(&[&workflow_name, &error])
			.set(count);
	}

	for (workflow_name, count) in sleeping_workflow_count {
		chirp_workflow::metrics::WORKFLOW_SLEEPING
			.with_label_values(&[&workflow_name])
			.set(count);
	}

	for (signal_name, count) in pending_signal_count {
		chirp_workflow::metrics::SIGNAL_PENDING
			.with_label_values(&[&signal_name])
			.set(count);
	}

	Ok(())
}

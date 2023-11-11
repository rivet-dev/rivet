use chirp_worker::prelude::*;
use proto::backend::pkg::*;
use redis::AsyncCommands;

#[derive(Debug, sqlx::FromRow)]
struct RunRow {
	region_id: Uuid,
	create_ts: i64,
	cleanup_ts: Option<i64>,
}

#[derive(Debug, sqlx::FromRow)]
struct RunMetaNomadRow {
	dispatched_job_id: Option<String>,
	node_id: Option<String>,
}

#[worker(name = "job-run-cleanup")]
async fn worker(ctx: &OperationContext<job_run::msg::cleanup::Message>) -> GlobalResult<()> {
	// NOTE: Idempotent

	let crdb = ctx.crdb().await?;

	let run_id = unwrap_ref!(ctx.run_id).as_uuid();

	let Some((run_row, run_meta_nomad_row)) = rivet_pools::utils::crdb::tx(&crdb, |tx| {
		Box::pin(update_db(ctx.clone(), ctx.ts(), run_id, tx))
	})
	.await?
	else {
		if ctx.req_dt() > util::duration::minutes(5) {
			tracing::error!("discarding stale message");
			return Ok(());
		} else {
			retry_bail!("run not found, may be race condition with insertion");
		}
	};

	tracing::info!("removing from cache");
	if matches!(
		run_meta_nomad_row,
		Some(RunMetaNomadRow {
			node_id: Some(_),
			..
		})
	) {
		ctx.redis_job()
			.await?
			.hdel(
				util_job::key::proxied_ports(run_row.region_id),
				run_id.to_string(),
			)
			.await?;
	}

	msg!([ctx] job_run::msg::cleanup_complete(run_id) {
		run_id: Some(run_id.into()),
	})
	.await?;

	Ok(())
}

#[tracing::instrument(skip_all)]
async fn update_db(
	ctx: OperationContext<job_run::msg::cleanup::Message>,
	now: i64,
	run_id: Uuid,
	tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> GlobalResult<Option<(RunRow, Option<RunMetaNomadRow>)>> {
	let run_row = sql_fetch_optional!(
		[ctx, RunRow]
		"
		SELECT region_id, create_ts, cleanup_ts
		FROM db_job_state.runs
		WHERE run_id = $1
		FOR UPDATE
		",
		run_id,
	)
	.await?;
	tracing::info!(?run_row, "run row");

	let Some(run_row) = run_row else {
		return Ok(None);
	};

	let run_meta_nomad_row = sql_fetch_optional!(
		[ctx, RunMetaNomadRow]
		"
		SELECT dispatched_job_id, node_id
		FROM db_job_state.run_meta_nomad
		WHERE run_id = $1
		FOR UPDATE
		",
		run_id,
	)
	.await?;
	tracing::info!(?run_meta_nomad_row, "run meta row");

	// Check if job has been dispatched already
	if let Some(run_meta_nomad) = &run_meta_nomad_row {
		if run_meta_nomad.dispatched_job_id.is_none()
			&& now - run_row.create_ts < util::duration::seconds(75)
		{
			// If the job is new, then there may be a race condition with
			// submitting the job to Nomad and writing the dispatched job ID to
			// the database.
			//
			// In this case, we'll fail and retry this later.
			//
			// There is a situation where the Nomad API returns an error and the
			// job ID is never written to the database.
			retry_bail!("potential race condition with starting nomad job")
		}
	}

	tracing::info!("deleting run");
	if run_row.cleanup_ts.is_none() {
		sql_execute!(
			[ctx]
			"UPDATE db_job_state.runs SET cleanup_ts = $2 WHERE run_id = $1",
			run_id,
			now,
		)
		.await?;
	}

	Ok(Some((run_row, run_meta_nomad_row)))
}

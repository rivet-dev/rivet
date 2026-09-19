use anyhow::{Context, Result};
use chrono::{TimeZone, Utc};
use futures_util::{StreamExt, stream};
use gas::db::debug::{DatabaseDebug, DeadWorkflow, RepairMode, RepairState, RepairVariant};
use rivet_term::console::style;
use rivet_util::Id;
use tabled::Tabled;

/// Longest error shown in the table before it is truncated.
const ERROR_MAX_CHARS: usize = 80;

#[derive(Tabled)]
struct DeadWorkflowRow {
	#[tabled(rename = "workflow id")]
	workflow_id: Id,
	name: String,
	error: String,
	repairs: String,
	#[tabled(rename = "created at")]
	created_at: String,
	#[tabled(rename = "died at")]
	died_at: String,
	#[tabled(skip)]
	death_ts: Option<i64>,
	#[tabled(skip)]
	create_ts: Option<i64>,
}

pub async fn execute(
	db: &dyn DatabaseDebug,
	names: &[&str],
	error_like: &[&str],
	parallelization: usize,
) -> Result<()> {
	rivet_term::status::progress("Scanning", "dead workflow index");

	let dead_workflows = db.list_dead_workflows(names, error_like).await?;

	if dead_workflows.is_empty() {
		rivet_term::status::success("No dead workflows found", "");
		return Ok(());
	}

	let progress = rivet_term::progress::bar("Inspecting", dead_workflows.len() as u64);

	let mut rows = stream::iter(dead_workflows)
		.map(|dead_workflow| {
			let progress = progress.clone();
			async move {
				let row = build_row(db, dead_workflow).await;
				progress.inc(1);
				row
			}
		})
		.buffer_unordered(parallelization.max(1))
		.collect::<Vec<_>>()
		.await
		.into_iter()
		.collect::<Result<Vec<_>>>()?;

	progress.finish_and_clear();

	// Latest first. Workflows that died before death timestamps were recorded have none, so they
	// sort last.
	rows.sort_by(|a, b| {
		b.death_ts
			.cmp(&a.death_ts)
			.then_with(|| b.create_ts.cmp(&a.create_ts))
			.then_with(|| a.repairs.cmp(&b.repairs))
	});

	rivet_term::status::success("Dead workflows", rows.len());
	rivet_term::format::table(rows);

	Ok(())
}

async fn build_row(db: &dyn DatabaseDebug, dead_workflow: DeadWorkflow) -> Result<DeadWorkflowRow> {
	let workflow_id = dead_workflow.workflow_id;

	// Read one workflow at a time rather than passing every id to a single `get_workflows` call.
	// That call reads its whole batch in one transaction and reads each workflow sequentially
	// inside it, so a batch this size would hold a transaction open past its five second limit.
	// Fanning out here keeps each transaction to one workflow and runs them alongside the repair
	// inspections, which are per workflow anyway.
	let workflow = DatabaseDebug::get_workflows(db, vec![workflow_id])
		.await?
		.into_iter()
		.next();
	let death_ts = workflow.as_ref().and_then(|workflow| workflow.death_ts);
	let create_ts = workflow.as_ref().map(|workflow| workflow.create_ts);

	let mut repairs = Vec::new();
	for variant in RepairVariant::ALL {
		// A workflow whose history cannot be inspected is still listed, it just has no repair.
		match db
			.inspect_workflow_repair(workflow_id, *variant, None)
			.await
		{
			Ok(inspection) => {
				if inspection.state == RepairState::Ready {
					match inspection.mode {
						RepairMode::Automatic => repairs.push(variant.to_string()),
						RepairMode::ManualOnly => repairs.push(format!("{variant} (manual)")),
					}
				}
			}
			Err(err) => {
				tracing::warn!(?workflow_id, %variant, ?err, "failed to inspect workflow repair");
			}
		}
	}

	Ok(DeadWorkflowRow {
		workflow_id,
		name: dead_workflow.workflow_name,
		error: truncate_error(&dead_workflow.error),
		repairs: repairs.join(", "),
		created_at: create_ts.map(format_ts).transpose()?.unwrap_or_default(),
		died_at: match death_ts {
			Some(death_ts) => format_ts(death_ts)?,
			None => style("unset").dim().to_string(),
		},
		death_ts,
		create_ts,
	})
}

/// Collapses the error onto one line and cuts it to fit a table cell.
fn truncate_error(error: &str) -> String {
	let error = error.split_whitespace().collect::<Vec<_>>().join(" ");

	if error.chars().count() > ERROR_MAX_CHARS {
		format!(
			"{}…",
			error.chars().take(ERROR_MAX_CHARS).collect::<String>()
		)
	} else {
		error
	}
}

fn format_ts(ts: i64) -> Result<String> {
	let datetime = Utc
		.timestamp_millis_opt(ts)
		.single()
		.context("invalid ts")?;

	Ok(datetime.format("%Y-%m-%d %H:%M:%S").to_string())
}

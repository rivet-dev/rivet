use anyhow::*;
use chrono::{Local, TimeZone};
use clap::ValueEnum;
use indoc::indoc;
use rivet_term::console::style;
use uuid::Uuid;

use super::{build_pool, KvPair};
use crate::{
	context::ProjectContext,
	tasks::db,
	utils::{self, colored_json, indent_string},
};

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[clap(rename_all = "kebab_case")]
pub enum SignalState {
	Acked,
	Pending,
}

#[derive(Debug, sqlx::FromRow)]
pub struct SignalRow {
	signal_id: Uuid,
	signal_name: String,
	tags: Option<serde_json::Value>,
	workflow_id: Option<Uuid>,
	create_ts: i64,
	body: serde_json::Value,
	ack_ts: Option<i64>,
}

pub async fn get_signal(ctx: &ProjectContext, signal_id: Uuid) -> Result<Option<SignalRow>> {
	let (pool, _port) = build_pool(ctx).await?;
	let mut conn = db::sqlx::get_conn(&pool).await?;

	let signal = sqlx::query_as::<_, SignalRow>(indoc!(
		"
		SELECT
			signal_id,
			signal_name,
			NULL AS tags,
			workflow_id,
			create_ts,
			body,
			ack_ts
		FROM db_workflow.signals
		WHERE signal_id = $1
		UNION ALL
		SELECT
			signal_id,
			signal_name,
			tags,
			NULL AS workflow_id,
			create_ts,
			body,
			ack_ts
		FROM db_workflow.tagged_signals
		WHERE signal_id = $1
		"
	))
	.bind(signal_id)
	.fetch_optional(&mut *conn)
	.await?;

	Ok(signal)
}

pub async fn find_signals(
	ctx: &ProjectContext,
	tags: Vec<KvPair>,
	workflow_id: Option<Uuid>,
	name: Option<String>,
	state: Option<SignalState>,
) -> Result<Vec<SignalRow>> {
	let (pool, _port) = build_pool(ctx).await?;
	let mut conn = db::sqlx::get_conn(&pool).await?;

	let mut query_str = indoc!(
		"
		SELECT
			signal_id,
			signal_name,
			NULL AS tags,
			workflow_id,
			create_ts,
			body,
			ack_ts
		FROM db_workflow.signals
		WHERE
			($1 IS NULL OR signal_name = $1) AND
			($2 IS NULL OR workflow_id = $2) AND
			silence_ts IS NULL AND
			-- Acked
			(NOT $3 OR ack_ts IS NOT NULL) AND
			-- Pending
			(NOT $4 OR ack_ts IS NULL)
		UNION ALL
		SELECT
			signal_id,
			signal_name,
			tags,
			NULL AS workflow_id,
			create_ts,
			body,
			ack_ts
		FROM db_workflow.tagged_signals
		WHERE
			($1 IS NULL OR signal_name = $1) AND
			silence_ts IS NULL AND
			-- Acked
			(NOT $3 OR ack_ts IS NOT NULL) AND
			-- Pending
			(NOT $4 OR ack_ts IS NULL)
		"
	)
	.to_string();

	// Procedurally add tags. We don't combine the tags into an object because we are comparing
	// strings with `->>` whereas with @> and `serde_json::Map` we would have to know the type of the input
	// given.
	for i in 0..tags.len() {
		let idx = i * 2 + 5;
		let idx2 = idx + 1;

		query_str.push_str(&format!(" AND tags->>${idx} = ${idx2}"));
	}

	query_str.push_str("LIMIT 100");

	// eprintln!(
	// 	"{query_str} {name:?} {workflow_id:?} {} {} {}",
	// 	tags.is_empty(),
	// 	matches!(state, Some(SignalState::Acked)),
	// 	matches!(state, Some(SignalState::Pending))
	// );

	let mut query = sqlx::query_as::<_, SignalRow>(&query_str)
		.bind(name)
		.bind(workflow_id)
		.bind(matches!(state, Some(SignalState::Acked)))
		.bind(matches!(state, Some(SignalState::Pending)));

	for tag in tags {
		query = query.bind(tag.key);
		query = query.bind(tag.value);
	}

	let signals = query.fetch_all(&mut *conn).await?;

	Ok(signals)
}

pub async fn print_signals(signals: Vec<SignalRow>, pretty: bool) -> Result<()> {
	if signals.is_empty() {
		rivet_term::status::success("No signals found", "");
		return Ok(());
	}

	rivet_term::status::success("Signals", signals.len());

	if pretty {
		for signal in signals {
			println!("");

			println!("{}", style(signal.signal_name).bold());

			println!("  {} {}", style("id").bold(), signal.signal_id);

			let datetime = Local
				.timestamp_millis_opt(signal.create_ts)
				.single()
				.context("invalid ts")?;
			let date = datetime.format("%Y-%m-%d %H:%M:%S");

			println!("  {} {}", style("created at").bold(), style(date).magenta());

			print!("  {} ", style("state").bold());
			if signal.ack_ts.is_some() {
				println!("{}", style("ack'd").bright().blue());
			} else {
				println!("{}", style("pending").yellow());
			}
			println!(
				"  {} {}",
				style("body").bold(),
				&indent_string(&colored_json(&signal.body)?, "    ")[4..]
			);
		}
	} else {
		render::signals(signals)?;
	}

	Ok(())
}

pub async fn silence_signal(ctx: &ProjectContext, signal_id: Uuid) -> Result<()> {
	let (pool, _port) = build_pool(ctx).await?;
	let mut conn = db::sqlx::get_conn(&pool).await?;

	sqlx::query(indoc!(
		"
		WITH
			update_signals AS (
				UPDATE db_workflow.signals
				SET silence_ts = $2
				WHERE signal_id = $1
				RETURNING 1
			),
			update_tagged_signals AS (
				UPDATE db_workflow.tagged_signals
				SET silence_ts = $2
				WHERE signal_id = $1
				RETURNING 1
			)
		SELECT 1
		"
	))
	.bind(signal_id)
	.bind(utils::now())
	.execute(&mut *conn)
	.await?;

	Ok(())
}

mod render {
	use anyhow::*;
	use rivet_term::console::style;
	use tabled::Tabled;
	use uuid::Uuid;

	use super::{SignalRow, SignalState};
	use crate::utils::colored_json_ugly;

	#[derive(Tabled)]
	struct SignalTableRow {
		pub signal_name: String,
		pub signal_id: Uuid,
		#[tabled(display_with = "display_state")]
		pub state: SignalState,
		#[tabled(rename = "tags/workflow_id")]
		pub id: String,
	}

	pub fn signals(signals: Vec<SignalRow>) -> Result<()> {
		let mut rows = signals
			.iter()
			.map(|w| {
				Ok(SignalTableRow {
					signal_name: w.signal_name.clone(),
					signal_id: w.signal_id,
					state: if w.ack_ts.is_some() {
						SignalState::Acked
					} else {
						SignalState::Pending
					},
					id: w
						.tags
						.as_ref()
						.map(colored_json_ugly)
						.transpose()?
						.or(w.workflow_id.map(|id| id.to_string()))
						.unwrap(),
				})
			})
			.collect::<Result<Vec<_>>>()?;

		rows.sort_by_key(|w| w.state);

		rivet_term::format::table(rows);

		Ok(())
	}

	fn display_state(state: &SignalState) -> String {
		match state {
			SignalState::Acked => style("ack'd").bright().blue().to_string(),
			SignalState::Pending => style("pending").yellow().to_string(),
		}
	}
}

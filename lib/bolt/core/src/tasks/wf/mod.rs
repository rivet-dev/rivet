use std::{
	cmp::Ordering,
	path::{Path, PathBuf},
	process::Stdio,
};

use anyhow::*;
use chrono::{TimeZone, Utc};
use clap::ValueEnum;
use duct::cmd;
use indoc::indoc;
use rivet_term::console::style;
use serde_json::json;
use sqlx::PgPool;
use tokio::{
	fs::{self, File},
	process::Command,
	task::block_in_place,
};
use uuid::Uuid;

use crate::{
	config,
	context::ProjectContext,
	dep,
	tasks::db,
	utils::{self, colored_json, db_conn::DatabaseConnections, indent_string},
};

pub mod signal;

#[derive(Debug, Clone)]
pub struct KvPair {
	pub key: String,
	pub value: String,
}

#[derive(thiserror::Error, Debug)]
pub enum ParseKvPairError {
	#[error("kv pairs must use `=` with no spaces (ex: foo=bar)")]
	NoEquals,
}

impl std::str::FromStr for KvPair {
	type Err = ParseKvPairError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let Some((key, value)) = s.split_once('=') else {
			return Result::Err(ParseKvPairError::NoEquals);
		};

		let key = key.trim().to_string();
		let value = value.trim().to_string();

		Result::Ok(KvPair { key, value })
	}
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[clap(rename_all = "kebab_case")]
pub enum WorkflowState {
	Complete,
	Running,
	Sleeping,
	Dead,
}

#[derive(strum::FromRepr)]
enum EventType {
	Activity = 0,
	Signal = 1,
	SubWorkflow = 2,
	SignalSend = 3,
	MessageSend = 4,
	Loop = 5,
}

impl EventType {
	fn name(&self) -> &str {
		match self {
			EventType::Activity => "activity",
			EventType::Signal => "signal receive",
			EventType::SubWorkflow => "sub workflow",
			EventType::SignalSend => "signal send",
			EventType::MessageSend => "message send",
			EventType::Loop => "loop",
		}
	}
}

#[derive(Debug, sqlx::FromRow)]
pub struct WorkflowRow {
	workflow_id: Uuid,
	workflow_name: String,
	tags: Option<serde_json::Value>,
	create_ts: i64,
	input: serde_json::Value,
	output: Option<serde_json::Value>,
	error: Option<String>,

	is_active: bool,
	has_wake_condition: bool,
}

#[derive(sqlx::FromRow)]
struct HistoryEventRow {
	location: Vec<i64>,
	/// Event type.
	t: i64,
	name: Option<String>,
	input: Option<serde_json::Value>,
	output: Option<serde_json::Value>,
	forgotten: bool,
}

#[derive(sqlx::FromRow)]
struct ActivityErrorRow {
	location: Vec<i64>,
	error: String,
	count: i64,
	latest_ts: i64,
}

pub async fn get_workflow(ctx: &ProjectContext, workflow_id: Uuid) -> Result<Option<WorkflowRow>> {
	let pool = build_pool(ctx).await?;
	let mut conn = db::sqlx::get_conn(&pool).await?;

	let workflow = sqlx::query_as::<_, WorkflowRow>(indoc!(
		"
		SELECT
			workflow_id,
			workflow_name,
			tags,
			create_ts,
			input,
			output,
			error,
			worker_instance_id IS NOT NULL AS is_active,
			(
				wake_immediate OR
				wake_deadline_ts IS NOT NULL OR
				cardinality(wake_signals) > 0 OR
				wake_sub_workflow_id IS NOT NULL
			) AS has_wake_condition
		FROM db_workflow.workflows
		WHERE
			workflow_id = $1
		"
	))
	.bind(workflow_id)
	.fetch_optional(&mut *conn)
	.await?;

	Ok(workflow)
}

pub async fn find_workflows(
	ctx: &ProjectContext,
	tags: Vec<KvPair>,
	name: Option<String>,
	state: Option<WorkflowState>,
) -> Result<Vec<WorkflowRow>> {
	let pool = build_pool(ctx).await?;
	let mut conn = db::sqlx::get_conn(&pool).await?;

	let mut query_str = indoc!(
		"
		SELECT
			workflow_id,
			workflow_name,
			tags,
			create_ts,
			input,
			output,
			error,
			worker_instance_id IS NOT NULL AS is_active,
			(
				wake_immediate OR
				wake_deadline_ts IS NOT NULL OR
				cardinality(wake_signals) > 0 OR
				wake_sub_workflow_id IS NOT NULL
			) AS has_wake_condition
		FROM db_workflow.workflows
		WHERE
			($1 IS NULL OR workflow_name = $1) AND
			silence_ts IS NULL AND
			-- Complete
			(NOT $2 OR output IS NOT NULL) AND
			-- Running
			(NOT $3 OR (
				output IS NULL AND
				worker_instance_id IS NOT NULL
			)) AND
			-- Sleeping
			(NOT $4 OR (
				output IS NULL AND
				worker_instance_id IS NULL AND
				(
					wake_immediate OR
					wake_deadline_ts IS NOT NULL OR
					cardinality(wake_signals) > 0 OR
					wake_sub_workflow_id IS NOT NULL
				)
			)) AND
			-- Dead
			(NOT $5 OR (
				output IS NULL AND
				worker_instance_id IS NULL AND
				wake_immediate = FALSE AND
				wake_deadline_ts IS NULL AND
				cardinality(wake_signals) = 0 AND
				wake_sub_workflow_id IS NULL
			))
		"
	)
	.to_string();

	// Procedurally add tags. We don't combine the tags into an object because we are comparing
	// strings with `->>` whereas with @> and `serde_json::Map` we would have to know the type of the input
	// given.
	for i in 0..tags.len() {
		let idx = i * 2 + 6;
		let idx2 = idx + 1;

		query_str.push_str(&format!(" AND tags->>${idx} = ${idx2}"));
	}

	query_str.push_str("LIMIT 100");

	let mut query = sqlx::query_as::<_, WorkflowRow>(&query_str)
		.bind(name)
		.bind(matches!(state, Some(WorkflowState::Complete)))
		.bind(matches!(state, Some(WorkflowState::Running)))
		.bind(matches!(state, Some(WorkflowState::Sleeping)))
		.bind(matches!(state, Some(WorkflowState::Dead)));

	for tag in tags {
		query = query.bind(tag.key);
		query = query.bind(tag.value);
	}

	let workflows = query.fetch_all(&mut *conn).await?;

	Ok(workflows)
}

pub async fn print_workflows(workflows: Vec<WorkflowRow>, pretty: bool) -> Result<()> {
	if workflows.is_empty() {
		rivet_term::status::success("No workflows found", "");
		return Ok(());
	}

	rivet_term::status::success("Workflows", workflows.len());

	if pretty {
		for workflow in workflows {
			println!("");

			println!("{}", style(workflow.workflow_name).bold());

			println!("  {} {}", style("id").bold(), workflow.workflow_id);

			let datetime = Utc
				.timestamp_millis_opt(workflow.create_ts)
				.single()
				.context("invalid ts")?;
			let date = datetime.format("%Y-%m-%d %H:%M:%S");

			println!("  {} {}", style("created at").bold(), style(date).magenta());

			print!("  {} ", style("state").bold());
			if workflow.output.is_some() {
				println!("{}", style("complete").bright().blue());
			} else if workflow.is_active {
				println!("{}", style("running").green());
			} else if workflow.has_wake_condition {
				println!("{}", style("sleeping").yellow());
			} else {
				println!("{}", style("dead").red());
			}

			if let Some(error) = workflow.error {
				println!(
					"  {} {}",
					style("error").bold(),
					style(&indent_string(&chunk_string(&error, 200).join("\n"), "    ")[4..])
						.green()
				);
			}

			println!(
				"  {} {}",
				style("input").bold(),
				&indent_string(&colored_json(&workflow.input)?, "    ")[4..]
			);

			print!("  {} ", style("output").bold());
			if let Some(output) = workflow.output {
				println!("{}", &indent_string(&colored_json(&output)?, "    ")[4..]);
			} else {
				println!("{}", style("<none>").dim());
			}
		}
	} else {
		render::workflows(workflows)?;
	}

	Ok(())
}

pub async fn silence_workflow(ctx: &ProjectContext, workflow_id: Uuid) -> Result<()> {
	let pool = build_pool(ctx).await?;
	let mut conn = db::sqlx::get_conn(&pool).await?;

	sqlx::query(indoc!(
		"
		UPDATE db_workflow.workflows
		SET silence_ts = $2
		WHERE workflow_id = $1
		"
	))
	.bind(workflow_id)
	.bind(utils::now())
	.execute(&mut *conn)
	.await?;

	Ok(())
}

pub async fn wake_workflow(ctx: &ProjectContext, workflow_id: Uuid) -> Result<()> {
	let pool = build_pool(ctx).await?;
	let mut conn = db::sqlx::get_conn(&pool).await?;

	sqlx::query(indoc!(
		"
		UPDATE db_workflow.workflows
		SET wake_immediate = TRUE
		WHERE workflow_id = $1
		"
	))
	.bind(workflow_id)
	.execute(&mut *conn)
	.await?;

	Ok(())
}

pub async fn print_history(
	ctx: &ProjectContext,
	workflow_id: Uuid,
	include_errors: bool,
	include_forgotten: bool,
	print_location: bool,
) -> Result<()> {
	let pool = build_pool(ctx).await?;
	let mut conn = db::sqlx::get_conn(&pool).await?;
	let mut conn2 = db::sqlx::get_conn(&pool).await?;
	let mut conn3 = db::sqlx::get_conn(&pool).await?;

	let (wf_row, events, error_rows) = tokio::try_join!(
		async move {
			sqlx::query_as::<_, (String, serde_json::Value, Option<serde_json::Value>)>(indoc!(
				"
				SELECT workflow_name, input, output
				FROM db_workflow.workflows
				WHERE workflow_id = $1
				"
			))
			.bind(workflow_id)
			.fetch_optional(&mut *conn)
			.await
			.map_err(Into::into)
		},
		async move {
			sqlx::query_as::<_, HistoryEventRow>(indoc!(
				"
				WITH workflow_events AS (
					SELECT $1 AS workflow_id
				)
				SELECT location, 0 AS t, activity_name AS name, input, output, forgotten
				FROM db_workflow.workflow_activity_events, workflow_events
				WHERE
					workflow_activity_events.workflow_id = workflow_events.workflow_id AND ($2 OR NOT forgotten)
				UNION ALL
				SELECT location, 1 AS t, signal_name AS name, body as input, null as output, forgotten
				FROM db_workflow.workflow_signal_events, workflow_events
				WHERE
					workflow_signal_events.workflow_id = workflow_events.workflow_id AND ($2 OR NOT forgotten)
				UNION ALL
				SELECT location, 2 AS t, w.workflow_name AS name, w.input, w.output, forgotten
				FROM workflow_events, db_workflow.workflow_sub_workflow_events AS sw
				JOIN db_workflow.workflows AS w
				ON sw.sub_workflow_id = w.workflow_id
				WHERE
					sw.workflow_id = workflow_events.workflow_id AND ($2 OR NOT forgotten)
				UNION ALL
				SELECT location, 3 AS t, signal_name AS name, body as input, null as output, forgotten
				FROM db_workflow.workflow_signal_send_events, workflow_events
				WHERE
					workflow_signal_send_events.workflow_id = workflow_events.workflow_id AND ($2 OR NOT forgotten)
				UNION ALL
				SELECT location, 4 AS t, message_name AS name, body as input, null as output, forgotten
				FROM db_workflow.workflow_message_send_events, workflow_events
				WHERE
					workflow_message_send_events.workflow_id = workflow_events.workflow_id AND ($2 OR NOT forgotten)
				UNION ALL
				SELECT location, 5 AS t, NULL AS name, null as input, null as output, forgotten
				FROM db_workflow.workflow_loop_events, workflow_events
				WHERE
					workflow_loop_events.workflow_id = workflow_events.workflow_id AND ($2 OR NOT forgotten)
				ORDER BY location ASC;
				"
			))
			.bind(workflow_id)
			.bind(include_forgotten)
			.fetch_all(&mut *conn2)
			.await
			.map_err(Into::into)
		},
		async move {
			if include_errors {
				sqlx::query_as::<_, ActivityErrorRow>(indoc!(
					"
					SELECT location, error, COUNT(error), MAX(ts) AS latest_ts
					FROM db_workflow.workflow_activity_errors
					WHERE workflow_id = $1
					GROUP BY location, error
					ORDER BY latest_ts
					"
				))
				.bind(workflow_id)
				.fetch_all(&mut *conn3)
				.await
				.map_err(Into::into)
			} else {
				Ok(Vec::new())
			}
		},
	)?;

	let Some((workflow_name, input, output)) = wf_row else {
		rivet_term::status::success("No workflow found", "");

		return Ok(());
	};

	// Print header
	{
		println!("");

		println!(
			"{} {}",
			style(workflow_name).yellow().bold(),
			style(workflow_id).yellow()
		);

		print!(
			"  {} {}",
			style("╰").yellow().dim(),
			style("input").yellow()
		);

		let input = serde_json::to_string(&input)?;
		let input_trim = input.chars().take(50).collect::<String>();
		print!(" {}", style(input_trim).yellow().dim());
		if input.len() > 50 {
			print!(" {}", style("...").yellow().dim());
		}

		println!("\n");
	}

	for i in 0..events.len() {
		let event = events.get(i).unwrap();
		let errors = error_rows
			.iter()
			.filter(|row| row.location == event.location)
			.collect::<Vec<_>>();

		let t = EventType::from_repr(event.t.try_into()?).context("invalid event type")?;

		// Indentation
		print!("{}", "  ".repeat(event.location.len().saturating_sub(1)));

		if print_location {
			print!("{} ", style(event.location.last().unwrap()).dim());
		}

		if event.forgotten {
			print!("{}", style(t.name()).red().dim().bold());
		} else {
			print!("{}", style(t.name()).bold());
		}

		if let Some(name) = &event.name {
			print!(" {}", style(name));
		}

		println!("");

		if let Some(input) = &event.input {
			print!("{}", "  ".repeat(event.location.len()));

			let c = if event.output.is_none() && errors.is_empty() {
				"╰"
			} else {
				"├"
			};
			let c = if event.forgotten {
				style(c).red().dim()
			} else {
				style(c).dim()
			};
			print!("{} ", c);

			print!("input");

			let input = serde_json::to_string(&input)?;
			let input_trim = input.chars().take(50).collect::<String>();
			print!(" {}", style(input_trim).dim());
			if input.len() > 50 {
				print!(" {}", style("...").dim());
			}

			println!("");
		}

		if let Some(output) = &event.output {
			print!("{}", "  ".repeat(event.location.len()));

			let c = if errors.is_empty() { "╰" } else { "├" };
			let c = if event.forgotten {
				style(c).red().dim()
			} else {
				style(c).dim()
			};
			print!("{} ", c);

			print!("output");

			let output = serde_json::to_string(&output)?;
			let output_trim = output.chars().take(50).collect::<String>();
			print!(" {}", style(output_trim).dim());
			if output.len() > 50 {
				print!(" {}", style("...").dim());
			}

			println!("");
		}

		if !errors.is_empty() {
			print!("{}", "  ".repeat(event.location.len()));

			if event.forgotten {
				print!("{} ", style("╰").red().dim());
			} else {
				print!("{} ", style("╰").dim());
			}

			println!("{}", style("errors"));

			for (i, error) in errors.iter().enumerate() {
				print!("{}", "  ".repeat(event.location.len() + 1));

				let c = if i == errors.len() - 1 { "╰" } else { "├" };
				let c = if event.forgotten {
					style(c).red().dim()
				} else {
					style(c).dim()
				};
				print!("{} ", c);

				let datetime = Utc
					.timestamp_millis_opt(error.latest_ts)
					.single()
					.context("invalid ts")?;
				let date = datetime.format("%Y-%m-%d %H:%M:%S");

				println!(
					"{} {} {}",
					style(format!("(last {})", date)).magenta(),
					style(format!("x{}", error.count)).yellow().bold(),
					style(error.error.replace('\n', " ")).dim(),
				);
			}
		}

		if !matches!(t, EventType::Loop) {
			println!("");
		}

		let next_event = events.get(i + 1);
		if let Some(ne) = next_event {
			match event.location.len().cmp(&ne.location.len()) {
				Ordering::Equal => {}
				Ordering::Less => {
					let start = if matches!(t, EventType::Loop) { 1 } else { 0 };

					for i in start..ne.location.len() - event.location.len() {
						print!(
							"{}",
							"  ".repeat(event.location.len().saturating_sub(1) + i)
						);

						if print_location {
							print!(
								"{} ",
								style(ne.location.get(ne.location.len() - 2).unwrap()).dim()
							);
						}

						if ne.forgotten {
							print!("{}", style("branch").red().dim().bold());
						} else {
							print!("{}", style("branch").bold());
						}

						println!();
					}
				}
				Ordering::Greater => {
					for (j, (a, b)) in event.location.iter().zip(ne.location.iter()).enumerate() {
						if a != b {
							if j + 1 != ne.location.len() {
								// Indentation
								print!("{}", "  ".repeat(j));

								if print_location {
									print!("{} ", style(b).dim());
								}

								if ne.forgotten {
									print!("{}", style("branch").red().dim().bold());
								} else {
									print!("{}", style("branch").bold());
								}

								println!();
							}

							break;
						}
					}
				}
			}
		}
	}

	// Print footer
	if let Some(output) = output {
		println!("{}", style("Workflow complete").yellow().bold());

		print!(
			"  {} {}",
			style("╰").yellow().dim(),
			style("output").yellow()
		);

		let output = serde_json::to_string(&output)?;
		let output_trim = output.chars().take(50).collect::<String>();
		print!(" {}", style(output_trim).yellow().dim());
		if output.len() > 50 {
			print!(" {}", style("...").yellow().dim());
		}

		println!("");
	}

	Ok(())
}

fn crt_path() -> PathBuf {
	Path::new("/tmp").join("rivet").join("crdb-ca.crt")
}

async fn build_pool(ctx: &ProjectContext) -> Result<PgPool> {
	if !utils::is_port_in_use(26257).await {
		bail!("to use `bolt wf` commands, you must manually start a new port forward with `bolt wf forward`.");
	}

	let db_workflow = ctx.service_with_name("db-workflow").await;
	let db_conn = DatabaseConnections::create(ctx, &[db_workflow], true).await?;

	let username = ctx.read_secret(&["crdb", "username"]).await?;
	let password = ctx.read_secret(&["crdb", "password"]).await?;
	let mut db_url = format!("postgres://{}:{}@localhost:26257", username, password);

	if let config::ns::ClusterKind::Distributed { .. } = ctx.ns().cluster.kind {
		// Add parameters
		db_url.push_str("?sslmode=verify-ca&sslrootcert=");
		db_url.push_str(&crt_path().to_str().context("bad path")?);

		let host = db_conn.cockroach_host.as_ref().unwrap();
		let (cluster_identifier, _) = host.split_once('.').context("bad crdb host url")?;
		db_url.push_str("&options=--cluster%3D");
		db_url.push_str(cluster_identifier);
	}

	// Must return port so it isn't dropped
	Ok(db::sqlx::build_pool(&db_url).await?)
}

pub async fn forward(ctx: &ProjectContext) -> Result<()> {
	match ctx.ns().cluster.kind {
		config::ns::ClusterKind::SingleNode { .. } => {
			let port =
				utils::kubectl_port_forward(ctx, "cockroachdb", "svc/cockroachdb", (26257, 26257))?;
			port.check().await?;

			rivet_term::status::progress("Proxying", "`bolt wf` commands can now be executed");

			port.wait().await?;
		}
		config::ns::ClusterKind::Distributed { .. } => {
			let db_workflow = ctx.service_with_name("db-workflow").await;
			let db_conn = DatabaseConnections::create(ctx, &[db_workflow], true).await?;

			let host = db_conn.cockroach_host.as_ref().unwrap();

			// Check if proxy pod exists
			let res = block_in_place(|| {
				cmd!(
					"kubectl",
					"get",
					"pod/crdb-proxy",
					"-n",
					"bolt",
					"--ignore-not-found"
				)
				.env("KUBECONFIG", ctx.gen_kubeconfig_path())
				.read()
			})?;
			let persistent_pod_exists = !res.is_empty();

			// Create TCP proxy through K8s cluster
			if !persistent_pod_exists {
				let spec = json!({
					"apiVersion": "v1",
					"kind": "Pod",
					"metadata": {
						"name": "crdb-proxy",
						"namespace": "bolt",
						"labels": {
							"app.kubernetes.io/name": "crdb-proxy",
						}
					},
					"spec": {
						"containers": [{
							"name": "crdb-proxy-container",
							"image": "alpine/socat",
							"args": [
								"TCP4-LISTEN:26257,fork,reuseaddr",
								format!("TCP4:{host}")
							],
							"ports": [{
								"containerPort": 26257
							}]
						}]
					}
				});

				rivet_term::status::progress("Applying spec", "pod/crdb-proxy");
				dep::k8s::cli::apply_specs(ctx, vec![spec]).await?;
			}

			let crt_path = crt_path();

			// Save CA cert locally
			if fs::metadata(&crt_path).await.is_err() {
				fs::create_dir_all(crt_path.parent().context("bad path")?).await?;

				let mut file = File::create(&crt_path).await?;

				// Read CRDB CA cert
				rivet_term::status::progress("Reading CRDB CA cert", "");
				let mut cmd = Command::new("kubectl")
					.args([
						"get",
						"configmap",
						"crdb-ca",
						"-n",
						"bolt",
						"-o",
						"jsonpath={.data.ca\\.crt}",
					])
					.env("KUBECONFIG", ctx.gen_kubeconfig_path())
					.stdout(Stdio::piped())
					.spawn()?;

				// Pipe to file
				if let Some(mut stdout) = cmd.stdout.take() {
					tokio::io::copy(&mut stdout, &mut file).await?;
				}

				ensure!(cmd.wait().await?.success());
			}
			let port = utils::kubectl_port_forward(ctx, "bolt", "pod/crdb-proxy", (26257, 26257))?;

			port.check().await?;

			rivet_term::status::progress("Proxying", "`bolt wf` commands can now be executed");

			port.wait().await?;
		}
	}

	Ok(())
}

fn chunk_string(s: &str, size: usize) -> Vec<String> {
	s.as_bytes()
		.chunks(size)
		.map(|chunk| String::from_utf8_lossy(chunk).to_string())
		.collect()
}

mod render {
	use anyhow::*;
	use rivet_term::console::style;
	use tabled::Tabled;
	use uuid::Uuid;

	use super::{WorkflowRow, WorkflowState};
	use crate::utils::colored_json_ugly;

	#[derive(Tabled)]
	struct WorkflowTableRow {
		pub workflow_id: Uuid,
		pub workflow_name: String,
		#[tabled(display_with = "display_state")]
		pub state: WorkflowState,
		#[tabled(display_with = "display_option")]
		pub tags: Option<String>,
	}

	pub fn workflows(workflows: Vec<WorkflowRow>) -> Result<()> {
		let mut rows = workflows
			.iter()
			.map(|w| {
				Ok(WorkflowTableRow {
					workflow_name: w.workflow_name.clone(),
					workflow_id: w.workflow_id,
					state: if w.output.is_some() {
						WorkflowState::Complete
					} else if w.is_active {
						WorkflowState::Running
					} else if w.has_wake_condition {
						WorkflowState::Sleeping
					} else {
						WorkflowState::Dead
					},
					tags: w.tags.as_ref().map(colored_json_ugly).transpose()?,
				})
			})
			.collect::<Result<Vec<_>>>()?;

		rows.sort_by_key(|w| w.state);

		rivet_term::format::table(rows);

		Ok(())
	}

	fn display_state(state: &WorkflowState) -> String {
		match state {
			WorkflowState::Complete => style("complete").bright().blue().to_string(),
			WorkflowState::Running => style("running").green().to_string(),
			WorkflowState::Sleeping => style("sleeping").yellow().to_string(),
			WorkflowState::Dead => style("dead").red().to_string(),
		}
	}

	pub(crate) fn display_option<T: std::fmt::Display>(item: &Option<T>) -> String {
		match item {
			Some(s) => s.to_string(),
			None => String::new(),
		}
	}
}

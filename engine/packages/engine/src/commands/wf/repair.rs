use std::time::Duration;

use anyhow::{Result, bail, ensure};
use gas::{
	db::debug::{
		DatabaseDebug, RepairCheck, RepairInspection, RepairMode, RepairState, RepairVariant,
		RepairVerification, RepairVerifyState,
	},
	history::location::Location,
};
use rivet_term::console::style;
use rivet_util::Id;

/// How long to wait for another worker to replay the repaired workflow before giving up and telling
/// the operator to re-run.
const VERIFY_ATTEMPTS: usize = 10;
const VERIFY_INTERVAL: Duration = Duration::from_secs(1);

pub async fn execute(
	db: &dyn DatabaseDebug,
	workflow_id: Id,
	variant: Option<RepairVariant>,
	location: Option<Location>,
	yes: bool,
	dry_run: bool,
) -> Result<()> {
	rivet_term::status::progress("Inspecting", workflow_id);

	// Every repair validates the history itself rather than trusting the persisted error, so run
	// all of them and let the structure decide which one applies. A caller that already knows which
	// defect this workflow has can name it, which is the only way to repair a workflow that also
	// matches a detect-only symptom.
	let variants = match &variant {
		Some(variant) => std::slice::from_ref(variant),
		None => RepairVariant::ALL,
	};

	let mut inspections = Vec::new();
	for variant in variants {
		inspections.push(
			db.inspect_workflow_repair(workflow_id, *variant, location.clone())
				.await?,
		);
	}

	print_workflow(&inspections);

	for inspection in &inspections {
		print_inspection(inspection);
	}
	eprintln!();

	// A detect-only symptom means the history is inconsistent in a way whose correct remedy depends
	// on which side is authoritative. Stop before offering to change anything, including any
	// automatic repair that also matched, because the workflow is not in a state this tool
	// understands well enough to write to. Naming a variant is the operator saying they already
	// read the history and decided, so only that variant is inspected and a detect-only symptom
	// they did not name no longer blocks the repair.
	let manual = inspections
		.iter()
		.filter(|inspection| {
			inspection.state == RepairState::Ready && inspection.mode == RepairMode::ManualOnly
		})
		.collect::<Vec<_>>();
	if !manual.is_empty() {
		print_manual(workflow_id, &manual);
		return Ok(());
	}

	let ready = inspections
		.iter()
		.filter(|inspection| {
			inspection.state == RepairState::Ready && inspection.mode == RepairMode::Automatic
		})
		.collect::<Vec<_>>();
	let applied = inspections
		.iter()
		.filter(|inspection| inspection.state == RepairState::AlreadyApplied)
		.collect::<Vec<_>>();

	// The repairs are mutually exclusive: one inserts an event, the other clears one. Matching both
	// means the history is in a state neither was designed for, so applying either could corrupt it
	// further.
	ensure!(
		ready.len() + applied.len() <= 1,
		"{} repairs match this workflow, refusing to guess which one is correct",
		ready.len() + applied.len(),
	);

	let pre_repair_error = inspections
		.first()
		.and_then(|inspection| inspection.workflow_error.clone());

	// Reported before the dry run bails so a dry run still says what it would have done.
	match (ready.first(), applied.first()) {
		(Some(inspection), _) => rivet_term::status::warn("Repair available", inspection.variant),
		(None, Some(inspection)) => rivet_term::status::warn(
			"Already repaired",
			format!("{} is in place, needs a wake", inspection.variant),
		),
		(None, None) => rivet_term::status::warn(
			"No repair available",
			match &variant {
				Some(variant) => {
					format!("this workflow does not match {variant}, see the failed checks above",)
				}
				None => {
					"this workflow does not match any known defect, see the failed checks above"
						.to_string()
				}
			},
		),
	}

	if dry_run {
		rivet_term::status::info("Dry run", "nothing was changed, drop --dry-run to repair");
		return Ok(());
	}

	let (variant, repair_location) = if let Some(inspection) = ready.first() {
		if !yes
			&& !confirm(format!(
				"Apply the {} repair to {workflow_id}?",
				inspection.variant
			))
			.await?
		{
			rivet_term::status::info("Aborted", "nothing was changed");
			return Ok(());
		}

		let outcome = db
			.repair_workflow(workflow_id, inspection.variant, inspection.location.clone())
			.await?;

		rivet_term::status::success("Repaired", workflow_id);
		for mutation in &outcome.mutations {
			eprintln!("  {mutation}");
		}

		// A repair that released a stranded lease already armed an immediate wake the same way
		// lease failover does. Waking again would double count the workflow state metrics.
		if outcome.wake_armed {
			rivet_term::status::success("Woke", "released stranded lease and armed immediate wake");
		} else {
			db.wake_workflows(vec![workflow_id]).await?;
			rivet_term::status::success("Woke", workflow_id);
		}

		(outcome.inspection.variant, outcome.inspection.location)
	} else if let Some(inspection) = applied.first() {
		if !yes && !confirm(format!("Wake {workflow_id} and verify?")).await? {
			rivet_term::status::info("Aborted", "nothing was changed");
			return Ok(());
		}

		db.wake_workflows(vec![workflow_id]).await?;
		rivet_term::status::success("Woke", workflow_id);

		(inspection.variant, inspection.location.clone())
	} else {
		return Ok(());
	};

	let Some(repair_location) = repair_location else {
		bail!("repair did not report a location to verify");
	};

	eprintln!();
	rivet_term::status::progress("Verifying", &repair_location);

	let mut attempts_left = VERIFY_ATTEMPTS;
	let verification = loop {
		let verification = db
			.verify_workflow_repair(workflow_id, variant, repair_location.clone())
			.await?;

		attempts_left -= 1;

		// A workflow keeps its last error until it next commits, so an error identical to the one
		// the inspection saw is the old one, not a new failure. Only the caller knows what it was
		// before, so the wait for a replay is decided here rather than in the verification.
		let error_unchanged = verification.workflow_error == pre_repair_error;
		let replaying =
			matches!(
				verification.state,
				RepairVerifyState::ReplayRunning | RepairVerifyState::AwaitingReplay
			) || (error_unchanged && !verification.has_lease && !verification.has_worker);

		if !replaying || attempts_left == 0 {
			break (verification, error_unchanged);
		}

		// There is no signal the CLI can wait on for a workflow being replayed by another process,
		// so poll for a bounded window and otherwise tell the operator to re-run.
		tokio::time::sleep(VERIFY_INTERVAL).await;
	};

	let (verification, error_unchanged) = verification;

	print_verification(&verification, error_unchanged);

	Ok(())
}

fn print_workflow(inspections: &[RepairInspection]) {
	let Some(inspection) = inspections.first() else {
		return;
	};

	eprintln!();
	eprintln!("  {} {}", style("workflow").bold(), inspection.workflow_id);
	eprintln!(
		"  {} {}",
		style("name").bold(),
		inspection.workflow_name.as_deref().unwrap_or("<not found>")
	);
	eprintln!(
		"  {} {}",
		style("error").bold(),
		style(inspection.workflow_error.as_deref().unwrap_or("<none>")).green()
	);
	eprintln!(
		"  {} lease={} worker={} wake_condition={}",
		style("state").bold(),
		inspection.has_lease,
		inspection.has_worker,
		inspection.has_wake_condition,
	);
}

fn print_manual(workflow_id: Id, manual: &[&RepairInspection]) {
	for inspection in manual {
		rivet_term::status::warn("Manual repair required", inspection.variant);

		if let Some(location) = &inspection.location {
			eprintln!("  {} {location}", style("location").bold());
		}
	}

	eprintln!();
	rivet_term::status::error(
		"Not repairing",
		"this workflow's history is inconsistent in a way that has more than one possible remedy",
	);
	eprintln!();
	eprintln!(
		"  The correct fix depends on which side of the inconsistency is authoritative, which this"
	);
	eprintln!("  tool cannot determine. Read the full history and decide by hand:");
	eprintln!();
	eprintln!(
		"    {}",
		style(format!("rivet-engine wf history {workflow_id} -fttl")).bold()
	);
	eprintln!();
	eprintln!("  Then send that output to Rivet along with the symptoms above.");
}

fn print_inspection(inspection: &RepairInspection) {
	eprintln!();
	eprintln!(
		"  {} {}",
		style(inspection.variant).bold(),
		match inspection.state {
			RepairState::Ready => style("ready").green(),
			RepairState::AlreadyApplied => style("already applied").yellow(),
			RepairState::NotApplicable => style("not applicable").dim(),
		}
	);

	for check in &inspection.checks {
		print_check(check);
	}
}

fn print_check(check: &RepairCheck) {
	let mark = if check.passed {
		style("✓").green()
	} else {
		style("✗").red()
	};

	eprintln!("    {mark} {} {}", style(&check.name).bold(), check.detail);
}

fn print_verification(verification: &RepairVerification, error_unchanged: bool) {
	for check in &verification.checks {
		print_check(check);
	}

	eprintln!();
	eprintln!(
		"  {} {}",
		style("error").bold(),
		style(verification.workflow_error.as_deref().unwrap_or("<none>")).green()
	);
	eprintln!(
		"  {} lease={} worker={} wake_condition={}",
		style("state").bold(),
		verification.has_lease,
		verification.has_worker,
		verification.has_wake_condition,
	);
	eprintln!();

	// The workflow still carries the error it had before the repair, so it has not committed since
	// and nothing about its state reflects the repair yet.
	if error_unchanged
		&& !matches!(verification.state, RepairVerifyState::Recovered)
		&& !verification.has_lease
		&& !verification.has_worker
	{
		rivet_term::status::warn(
			"Awaiting replay",
			"the repair is in place but the workflow has not replayed yet, re-run this command to check again",
		);

		return;
	}

	match verification.state {
		RepairVerifyState::Recovered => rivet_term::status::success(
			"Recovered",
			format!(
				"{} replayed past the repair, also confirm the actor through the actor API",
				verification.workflow_id
			),
		),
		RepairVerifyState::ReplayRunning => rivet_term::status::warn(
			"Replaying",
			"the workflow is still replaying, re-run this command to check again",
		),
		RepairVerifyState::AwaitingReplay => rivet_term::status::warn(
			"Awaiting replay",
			"the repair is in place but the workflow has not replayed yet, re-run this command to check again",
		),
		RepairVerifyState::Regressed => rivet_term::status::error(
			"Regressed",
			"the defect came back after the workflow replayed, the underlying race fired again and this repair is not the fix",
		),
		RepairVerifyState::UnrelatedError => rivet_term::status::error(
			"Unrelated error",
			"the repair is in place and the workflow replayed, but it died with an error this tool does not handle",
		),
	}
}

async fn confirm(message: impl Into<String>) -> Result<bool> {
	let term = rivet_term::terminal();

	// The prompt loops until it reads a valid answer, and a closed stdin reads as an endless run of
	// empty answers. Refuse up front rather than spinning.
	ensure!(
		term.is_term(),
		"cannot ask for confirmation without a terminal, pass --yes to apply or --dry-run to only inspect",
	);

	let confirmed = rivet_term::prompt::PromptBuilder::default()
		.message(message)
		.build()?
		.bool(&term)
		.await?;

	Ok(confirmed)
}

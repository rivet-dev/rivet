use anyhow::*;
use chrono::{Local, TimeZone};
use rivet_term::console::style;

use chirp_workflow::db::debug::{SignalData, SignalState};

use crate::util::format::{colored_json, indent_string};

pub async fn print_signals(signals: Vec<SignalData>, pretty: bool) -> Result<()> {
	if signals.is_empty() {
		rivet_term::status::success("No signals found", "");
		return Ok(());
	}

	rivet_term::status::success("Signals", signals.len());

	if pretty {
		for signal in signals {
			println!();

			println!("{}", style(signal.signal_name).bold());

			println!("  {} {}", style("id").bold(), signal.signal_id);

			let datetime = Local
				.timestamp_millis_opt(signal.create_ts)
				.single()
				.context("invalid ts")?;
			let date = datetime.format("%Y-%m-%d %H:%M:%S");

			println!("  {} {}", style("created at").bold(), style(date).magenta());

			print!("  {} ", style("state").bold());
			match signal.state {
				SignalState::Acked => println!("{}", style("ack'd").bright().blue()),
				SignalState::Pending => println!("{}", style("pending").yellow()),
			}
			println!(
				"  {} {}",
				style("body").bold(),
				&indent_string(&colored_json(&signal.body)?, "    ", true)
			);
		}
	} else {
		table::signals(signals)?;
	}

	Ok(())
}

mod table {
	use anyhow::*;
	use chirp_workflow::db::debug::{SignalData, SignalState};
	use rivet_term::console::style;
	use tabled::Tabled;
	use uuid::Uuid;

	use crate::util::format::colored_json_ugly;

	#[derive(Tabled)]
	struct SignalTableRow {
		pub signal_name: String,
		pub signal_id: Uuid,
		#[tabled(display_with = "display_state")]
		pub state: SignalState,
		#[tabled(rename = "tags/workflow_id")]
		pub id: String,
	}

	pub fn signals(signals: Vec<SignalData>) -> Result<()> {
		let mut rows = signals
			.iter()
			.map(|s| {
				Ok(SignalTableRow {
					signal_name: s.signal_name.clone(),
					signal_id: s.signal_id,
					state: s.state,
					id: s
						.tags
						.as_ref()
						.map(colored_json_ugly)
						.transpose()?
						.or(s.workflow_id.map(|id| id.to_string()))
						.unwrap(),
				})
			})
			.collect::<Result<Vec<_>>>()?;

		rows.sort_by_key(|s| s.state);

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

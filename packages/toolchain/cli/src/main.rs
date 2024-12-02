pub mod commands;
pub mod util;

use clap::{builder::styling, Parser};
use std::process::ExitCode;
use toolchain::errors;

const STYLES: styling::Styles = styling::Styles::styled()
	.header(styling::AnsiColor::Red.on_default().bold())
	.usage(styling::AnsiColor::Red.on_default().bold())
	.literal(styling::AnsiColor::White.on_default().bold())
	.placeholder(styling::AnsiColor::White.on_default());

#[derive(Parser)]
#[clap(
	author = "Rivet Gaming, Inc. <developer@rivet.gg>",
	about = "https://rivet.gg/",
	version = concat!(env!("CARGO_PKG_VERSION"), " (", env!("VERGEN_GIT_SHA"), ")"),
	long_version = concat!(
		"\n\n",
		"git sha: ", env!("VERGEN_GIT_SHA"), "\n",
		"git branch: ", env!("VERGEN_GIT_BRANCH"), "\n",
		"build semver: ", env!("CARGO_PKG_VERSION"), "\n",
		"build timestamp: ", env!("VERGEN_BUILD_TIMESTAMP"), "\n",
		"build target: ", env!("VERGEN_CARGO_TARGET_TRIPLE"), "\n",
		"build debug: ", env!("VERGEN_CARGO_DEBUG"), "\n",
		"rustc version: ", env!("VERGEN_RUSTC_SEMVER"),
	),
    styles = STYLES
)]

struct Cli {
	#[command(subcommand)]
	command: commands::SubCommand,
}

fn main() -> ExitCode {
	// We use a sync main for Sentry. Read more: https://docs.sentry.io/platforms/rust/#async-main-function

	// This has a 2 second deadline to flush any remaining events which is sufficient for
	// short-lived commands.
	let _guard = sentry::init(("https://b329eb15c63e1002611fb3b7a58a1dfa@o4504307129188352.ingest.us.sentry.io/4508361147809792", sentry::ClientOptions {
    release: sentry::release_name!(),
    ..Default::default()
}));

	// Run main
	let exit_code = tokio::runtime::Builder::new_multi_thread()
		.enable_all()
		.build()
		.unwrap()
		.block_on(async move { main_async().await });

	exit_code
}

async fn main_async() -> ExitCode {
	let cli = Cli::parse();
	let exit_code = match cli.command.execute().await {
		Ok(()) => ExitCode::SUCCESS,
		Err(err) => {
			// TODO(TOOL-438): Catch 400 API errors as user errors
			if err.is::<errors::GracefulExit>() || err.is::<errors::CtrlC>() {
				// Don't print anything, already handled
			} else if let Some(err) = err.downcast_ref::<errors::UserError>() {
				// Don't report error since this is a user error
				eprintln!("\n{err}");
			} else {
				// This is an internal error, report error
				eprintln!("\n{err}");
				report_error(err).await;
			}

			ExitCode::FAILURE
		}
	};

	// Wait for telemetry to publish
	util::telemetry::wait_all().await;

	exit_code
}

async fn report_error(err: anyhow::Error) {
	let event_id = sentry::integrations::anyhow::capture_anyhow(&err);

	// Capture event in PostHog
	util::telemetry::capture_event(
        "$exception",
        Some(|event: &mut async_posthog::Event| {
            event.insert_prop("errors", format!("{}", err))?;
            event.insert_prop("$sentry_event_id", event_id.to_string())?;
            event.insert_prop("$sentry_url", format!("https://sentry.io/organizations/rivet-gaming/issues/?project=4508361147809792&query={event_id}"))?;
            Ok(())
        }),
    )
    .await;
}

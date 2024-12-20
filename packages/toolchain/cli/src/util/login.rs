use anyhow::Result;
use inquire::validator::Validation;
use std::result::Result as StdResult;
use toolchain::tasks;

use crate::util::{
	os,
	task::{run_task, TaskOutputStyle},
};

pub fn inquire_self_hosting() -> Result<Option<String>> {
	let using_cloud = inquire::Confirm::new("Are you using Rivet Cloud?")
		.with_default(true)
		.prompt()?;

	let api_endpoint = if !using_cloud {
		let e = inquire::Text::new("What is the API endpoint?")
			.with_default("http://localhost:8080")
			.with_validator(|input: &str| match url::Url::parse(input) {
				Result::Ok(_) => StdResult::Ok(Validation::Valid),
				Err(err) => StdResult::Ok(Validation::Invalid(format!("{err}").into())),
			})
			.prompt()?;

		Some(e)
	} else {
		None
	};

	Ok(api_endpoint)
}

pub async fn login(api_endpoint: Option<String>) -> Result<()> {
	let api_endpoint = api_endpoint.unwrap_or_else(|| "https://api.rivet.gg".to_string());

	// Check if linked
	let output = run_task::<tasks::auth::check_state::Task>(
		TaskOutputStyle::None,
		tasks::auth::check_state::Input {},
	)
	.await?;
	if output.signed_in {
		eprintln!("Already logged in. Log out with `rivet logout`.");
		return Ok(());
	}

	// Start device link
	let device_link_output = run_task::<tasks::auth::start_sign_in::Task>(
		TaskOutputStyle::None,
		tasks::auth::start_sign_in::Input {
			api_endpoint: api_endpoint.clone(),
		},
	)
	.await?;

	// Open link in browser
	//
	// Linux root users often cannot open the browser, so we fallback to printing the URL
	if !os::is_linux_and_root()
		&& webbrowser::open_browser_with_options(
			webbrowser::Browser::Default,
			&device_link_output.device_link_url,
			webbrowser::BrowserOptions::new().with_suppress_output(true),
		)
		.is_ok()
	{
		println!(
			"Waiting for browser...\n\nIf browser did not open, open this URL to login:\n{}",
			device_link_output.device_link_url
		);
	} else {
		println!(
			"Open this URL to login:\n{}",
			device_link_output.device_link_url
		);
	}

	// Wait for finish
	run_task::<tasks::auth::wait_for_sign_in::Task>(
		TaskOutputStyle::None,
		tasks::auth::wait_for_sign_in::Input {
			api_endpoint: api_endpoint.clone(),
			device_link_token: device_link_output.device_link_token,
		},
	)
	.await?;
	eprintln!("Logged in");

	Ok(())
}

use std::path::Path;

use anyhow::{ensure, Result};
use tokio::process::Command;

use crate::{context::ProjectContext, utils::command_helper::CommandHelper};

pub enum BuildMethod {
	Native,
	Musl,
}

pub struct BuildOpts<'a, T: AsRef<str>> {
	pub build_calls: Vec<BuildCall<'a, T>>,
	pub build_method: BuildMethod,
	pub release: bool,
	/// How many threads to run in parallel when building.
	pub jobs: Option<usize>,
}

pub struct BuildCall<'a, T: AsRef<str>> {
	pub path: &'a Path,
	pub bins: &'a [T],
}

pub async fn build<'a, T: AsRef<str>>(ctx: &ProjectContext, opts: BuildOpts<'a, T>) -> Result<()> {
	let jobs_flag = if let Some(jobs) = opts.jobs {
		format!("--jobs {jobs}")
	} else {
		String::new()
	};

	let format_flag = if let Some(fmt) = &ctx.config_local().rust.message_format {
		format!("--message-format={fmt}")
	} else {
		String::new()
	};

	let release_flag = if opts.release { "--release" } else { "" };

	let user_id = {
		let mut cmd = std::process::Command::new("id");
		cmd.arg("-u");
		cmd.exec_string().await.unwrap().trim().to_owned()
	};
	let user_group = {
		let mut cmd = std::process::Command::new("id");
		cmd.arg("-g");
		cmd.exec_string().await.unwrap().trim().to_owned()
	};

	let build_calls = opts
		.build_calls
		.iter()
		.map(|build_call| {
			let path = build_call.path.display();
			let bin_flags = build_call
				.bins
				.iter()
				.map(|x| format!("--bin {}", x.as_ref()))
				.collect::<Vec<String>>()
				.join(" ");

			// TODO: Not sure why the .cargo/config.toml isn't working with nested projects, have to hardcode
			// the target dir
			indoc::formatdoc!(
				"
				if [ $? -eq 0 ]; then
					(cd {path} && cargo build {jobs_flag} {format_flag} {release_flag} {bin_flags} --target-dir $TARGET_DIR)
				fi
				"
			)
		})
		.collect::<Vec<_>>()
		.join("\n");

	// Generate build script
	let build_script = indoc::formatdoc!(
		r#"
		TARGET_DIR=$(readlink -f ./target)
		# Used for Tokio Console. See https://github.com/tokio-rs/console#using-it
		export RUSTFLAGS="--cfg tokio_unstable"
		# Used for debugging
		# export CARGO_LOG=cargo::core::compiler::fingerprint=info

		{build_calls}

		EXIT_CODE=$?
		"#,
	);

	// Execute build command
	match opts.build_method {
		BuildMethod::Native => {
			let mut cmd = Command::new("sh");
			cmd.current_dir(ctx.path());
			cmd.arg("-c");
			cmd.arg(indoc::formatdoc!(
				r#"
				{build_script}

				# Exit
				exit $EXIT_CODE
				"#
			));
			let status = cmd.status().await?;

			ensure!(status.success());
		}
		BuildMethod::Musl => {
			let mut cmd = Command::new("docker");
			cmd.arg("run");
			cmd.arg("-v").arg("cargo-cache:/root/.cargo/registry");
			cmd.arg("-v")
				.arg(format!("{}:/volume", ctx.path().display()));
			cmd.arg("--rm")
				.arg("--interactive")
				.arg("--tty")
				.arg("clux/muslrust:1.65.0-stable");
			cmd.arg("sh").arg("-c").arg(indoc::formatdoc!(
				r#"
				# HACK: Link musl-g++
				#
				# See https://github.com/emk/rust-musl-builder/issues/53#issuecomment-421806898
				ln -s $(which g++) /usr/local/bin/musl-g++

				{build_script}

				# Fix permissions of target folder no matter the exit status
				chown -R "{user_id}:{user_group}" ./target

				# Exit
				exit $EXIT_CODE
				"#
			));
			let status = cmd.status().await?;

			ensure!(status.success());
		}
	}

	Ok(())
}

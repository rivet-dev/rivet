use std::path::{Path, PathBuf};

use anyhow::{ensure, Context, Result};
use indoc::formatdoc;
use regex::Regex;
use serde_json::json;
use tokio::{fs, io::AsyncReadExt, process::Command, task::block_in_place};

use crate::{config, context::ProjectContext};

pub struct BuildOpts<'a, T: AsRef<str>> {
	pub build_calls: Vec<BuildCall<'a, T>>,
	/// Builds for release mode.
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

	let build_calls =
		opts.build_calls
			.iter()
			.map(|build_call| {
				let path = build_call.path.display();
				let bin_flags = build_call
					.bins
					.iter()
					.map(|x| format!("--bin {}", x.as_ref()))
					.collect::<Vec<String>>()
					.join(" ");

				format!("(cd {path} && cargo build {jobs_flag} {format_flag} {release_flag} {bin_flags})")
			})
			.collect::<Vec<_>>()
			.join("\n");

	let sccache_env = if let Some(sccache) = &ctx.ns().rust.sccache {
		formatdoc!(
			"
			export RUSTC_WRAPPER=sccache
			export SCCACHE_BUCKET='{bucket}'
			export SCCACHE_ENDPOINT='{endpoint}'
			export SCCACHE_REGION='{region}'
			export AWS_ACCESS_KEY_ID='{aws_access_key_id}'
			export AWS_SECRET_ACCESS_KEY='{aws_secret_access_key}'
			",
			bucket = sccache.bucket,
			endpoint = sccache.endpoint,
			region = sccache.region,
			aws_access_key_id = ctx.read_secret(&["sccache", "aws_access_key_id"]).await?,
			aws_secret_access_key = ctx
				.read_secret(&["sccache", "aws_secret_access_key"])
				.await?,
		)
	} else {
		String::new()
	};

	let build_script_path = ctx.gen_path().join("build_script.sh");
	let build_script_path_relative = build_script_path
		.strip_prefix(ctx.path())
		.context("failed to strip prefix")?;

	// TODO: Not sure why the .cargo/config.toml isn't working with nested projects, have to hardcode
	// the target dir
	// Generate build script
	let build_script = formatdoc!(
		r#"
		#!/bin/bash
		set -euf

		[ -z "${{CARGO_TARGET_DIR+x}}" ] && export CARGO_TARGET_DIR=$(readlink -f ./target)
		# Used for Tokio Console. See https://github.com/tokio-rs/console#using-it
		export RUSTFLAGS="--cfg tokio_unstable"
		# Used for debugging
		export CARGO_LOG=cargo::core::compiler::fingerprint=info

		{sccache_env}

		{build_calls}
		"#,
	);

	// Write build script to file
	fs::write(&build_script_path, build_script).await?;

	// Execute build command
	match &ctx.ns().cluster.kind {
		config::ns::ClusterKind::SingleNode { .. } => {
			// Make build script executable
			let mut cmd = Command::new("chmod");
			cmd.current_dir(ctx.path());
			cmd.arg("+x");
			cmd.arg(build_script_path.display().to_string());
			let status = cmd.status().await?;
			ensure!(status.success());

			// Execute
			let mut cmd = Command::new(build_script_path.display().to_string());
			cmd.current_dir(ctx.path());
			let status = cmd.status().await?;
			ensure!(status.success());
		}
		config::ns::ClusterKind::Distributed { .. } => {
			let optimization = if opts.release { "release" } else { "debug" };
			let repo = &ctx.ns().docker.repository;
			ensure!(repo.ends_with('/'), "docker repository must end with slash");
			let source_hash = ctx.source_hash();

			// Create directory for docker files
			let gen_path = ctx.gen_path().join("docker");
			fs::create_dir_all(&gen_path).await?;

			// Build all of the base binaries in batch to optimize build speed
			//
			// We could do this as a single multi-stage Docker container, but it requires
			// re-hashing the entire project every time to check the build layers and can be
			// faulty sometimes.
			let build_image_tag = {
				let image_tag = format!("{repo}build:{source_hash}");
				let dockerfile_path = gen_path.join(format!("Dockerfile.build"));

				// TODO: Use --secret to pass sccache credentials instead of the build script.
				fs::write(
					&dockerfile_path,
					formatdoc!(
						r#"
						# syntax=docker/dockerfile:1.2

						FROM rust:1.77.2-slim

						RUN apt-get update && apt-get install -y protobuf-compiler pkg-config libssl-dev g++

						RUN apt-get install --yes libpq-dev wget
						RUN wget https://github.com/mozilla/sccache/releases/download/v0.2.15/sccache-v0.2.15-x86_64-unknown-linux-musl.tar.gz \
							&& tar xzf sccache-v0.2.15-x86_64-unknown-linux-musl.tar.gz \
							&& mv sccache-v0.2.15-x86_64-unknown-linux-musl/sccache /usr/local/bin/sccache \
							&& chmod +x /usr/local/bin/sccache

						WORKDIR /usr/rivet
						
						COPY . .
						COPY {build_script_path} build_script.sh
						
						RUN chmod +x ./build_script.sh
						RUN \
							--mount=type=cache,target=/usr/rivet/target \
							--mount=type=cache,target=/usr/rivet/oss/target \
							sh -c ./build_script.sh
						
						# Copy all binaries from target directory (it is not included in the output because of cache mount)
						RUN \
							--mount=type=cache,target=/usr/rivet/target \
							--mount=type=cache,target=/usr/rivet/oss/target \
							find target/{optimization} -maxdepth 1 -type f ! -name "*.*" -exec cp {{}} /usr/bin/ \;
						"#,
						build_script_path = build_script_path_relative.display(),
					),
				)
				.await?;

				// Build image
				let mut cmd = Command::new("docker");
				cmd.current_dir(ctx.path());
				cmd.arg("build");
				cmd.arg("-f").arg(dockerfile_path);
				// Prints plain console output for debugging
				// cmd.arg("--progress=plain");
				cmd.arg("-t").arg(&image_tag);
				cmd.arg(".");

				let status = cmd.status().await?;
				ensure!(status.success());

				image_tag
			};

			for call in &opts.build_calls {
				for bin in call.bins {
					let bin = bin.as_ref();

					// Resolve the symlink for the svc_scripts dir since Docker does not resolve
					// symlinks in COPY commands
					let infra_path = ctx.path().join("infra");
					let infra_path_resolved = tokio::fs::read_link(&infra_path)
						.await
						.map_or_else(|_| infra_path.clone(), |path| ctx.path().join(path));
					let svc_scripts_path = infra_path_resolved.join("misc").join("svc_scripts");
					let svc_scripts_path_relative = svc_scripts_path
						.strip_prefix(ctx.path())
						.context("failed to strip prefix")?;

					// Build the final image
					let image_tag = format!("{repo}{bin}:{source_hash}");
					let dockerfile_path = gen_path.join(format!("Dockerfile.{bin}"));
					fs::write(
						&dockerfile_path,
						formatdoc!(
							r#"
							FROM {build_image_tag} AS build

							FROM debian:12.1-slim AS run

							# Update ca-certificates. Install curl for health checks.
							RUN DEBIAN_FRONTEND=noninteractive apt-get update -y && apt-get install -y --no-install-recommends ca-certificates openssl curl

							# Copy supporting scripts
							COPY {svc_scripts_path}/health_check.sh {svc_scripts_path}/install_ca.sh /usr/bin/
							RUN chmod +x /usr/bin/health_check.sh /usr/bin/install_ca.sh

							# Copy final binary
							COPY --from=build /usr/bin/{bin} /usr/bin/{bin}
							"#,
							svc_scripts_path = svc_scripts_path_relative.display(),
						),
					)
					.await?;

					// Build image
					let mut cmd = Command::new("docker");
					cmd.current_dir(ctx.path());
					cmd.arg("build");
					cmd.arg("-f").arg(dockerfile_path);
					// Prints plain console output for debugging
					// cmd.arg("--progress=plain");
					cmd.arg("-t").arg(image_tag);
					cmd.arg(".");

					let status = cmd.status().await?;
					ensure!(status.success());
				}
			}
		}
	}

	Ok(())
}

pub struct BuildTestOpts<'a, T: AsRef<str>> {
	pub build_calls: Vec<BuildTestCall<'a, T>>,
	/// How many threads to run in parallel when building.
	pub jobs: Option<usize>,
	pub test_filters: &'a [String],
}

pub struct BuildTestCall<'a, T: AsRef<str>> {
	pub path: &'a Path,
	pub packages: &'a [T],
}

#[derive(Debug)]
pub struct TestBinary {
	pub package: String,
	pub target: String,
	pub path: PathBuf,
	pub test_name: String,
}

pub async fn build_tests<'a, T: AsRef<str>>(
	ctx: &ProjectContext,
	opts: BuildTestOpts<'a, T>,
) -> Result<Vec<TestBinary>> {
	let mut test_binaries = vec![];
	for build_call in opts.build_calls {
		let abs_path = ctx.path().join(build_call.path);

		// Build command
		let mut cmd = Command::new("cargo");
		cmd.args(&[
			"test",
			"--no-run",
			"--message-format=json-render-diagnostics",
		])
		.stdout(std::process::Stdio::piped())
		.current_dir(abs_path)
		// Used for Tokio Console. See https://github.com/tokio-rs/console#using-it
		.env("RUSTFLAGS", "--cfg tokio_unstable");
		if let Some(jobs) = opts.jobs {
			cmd.args(&["--jobs", &jobs.to_string()]);
		}
		for test in build_call.packages {
			cmd.args(&["--package", test.as_ref()]);
		}
		if std::env::var("CARGO_TARGET_DIR").is_err() {
			cmd.env("CARGO_TARGET_DIR", ctx.cargo_target_dir());
		}
		let mut child = cmd.spawn()?;

		// Capture stdout
		let mut stdout = child.stdout.take().context("missing stdout")?;
		let mut stdout_str = String::new();
		stdout.read_to_string(&mut stdout_str).await?;

		// Wait for finish
		let status = child.wait().await?;
		ensure!(status.success(), "build test failed");

		// Parse artifacts
		let test_count_re = Regex::new(r"(?m)^(.*): test$").unwrap();
		for line in stdout_str.lines() {
			let v = serde_json::from_str::<serde_json::Value>(line).context("invalid json")?;
			if v["reason"] == "compiler-artifact" && v["target"]["kind"] == json!(["test"]) {
				if let Some(executable) = v["filenames"][0].as_str() {
					// Parsing the cargo package name (foo-bar) from
					// path+file:///foo/bar#foo-bar@0.0.1
					let package = v["package_id"]
						.as_str()
						.context("missing package_id")?
						.split_once("#")
						.context("split_once failed")?
						.1
						.split_once("@")
						.context("split_once failed")?
						.0;

					let target = v["target"]["name"]
						.as_str()
						.context("missing target name")?;

					// Parse the test count from the binary
					let test_list_args = [
						&["--list".to_string(), "--format".into(), "terse".into()],
						opts.test_filters,
					]
					.concat();
					let test_list_stdout =
						block_in_place(|| duct::cmd(executable, &test_list_args).read())?;

					// Run a test container for every test in the binary
					for cap in test_count_re.captures_iter(&test_list_stdout) {
						let test_name = &cap[1];
						test_binaries.push(TestBinary {
							package: package.to_string(),
							target: target.to_string(),
							path: PathBuf::from(executable),
							test_name: test_name.to_string(),
						})
					}
				}
			}
		}
	}

	Ok(test_binaries)
}

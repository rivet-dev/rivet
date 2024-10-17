use std::path::{Path, PathBuf};

use anyhow::{ensure, Context, Result};
use indoc::{formatdoc, indoc};
use regex::Regex;
use serde_json::json;
use tokio::{fs, process::Command, task::block_in_place};
use uuid::Uuid;

use crate::context::ProjectContext;

// TODO: Clean this up
const DOCKERIGNORE: &str = indoc!(
	r#"
	*

	!Bolt.toml
	!Cargo.lock
	!Cargo.toml
	!errors
	!gen/build_script.sh
	!gen/docker
	!gen/test_build_script.sh
	!infra/default-builds
	!infra/misc/svc_scripts
	!lib
	!proto
	!sdks/full/rust/Cargo.toml
	!sdks/full/rust/src
	!src
	!svc

	sdks/runtime
	svc/**/*.md
	"#
);

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
		export CARGO_TERM_COLOR=always

		{sccache_env}

		{build_calls}
		"#,
	);

	// Write build script to file
	fs::write(&build_script_path, build_script).await?;

	// Execute build command
	if ctx.build_svcs_locally() {
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
	} else {
		let optimization = if opts.release { "release" } else { "debug" };
		// Get repo to push to
		let (push_repo, _) = ctx.docker_repos().await;
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
			let image_tag = format!("{push_repo}build:{source_hash}");
			let dockerfile_path = gen_path.join(format!("Dockerfile.build"));

			// TODO: Use --secret to pass sccache credentials instead of the build script.
			fs::write(
				&dockerfile_path,
				formatdoc!(
					r#"
					# syntax=docker/dockerfile:1.2

					FROM rust:1.81.0-slim AS rust

					RUN apt-get update \
					    && apt-get install --yes protobuf-compiler pkg-config libssl-dev g++ git libpq-dev wget \
					    && wget https://github.com/mozilla/sccache/releases/download/v0.2.15/sccache-v0.2.15-x86_64-unknown-linux-musl.tar.gz \
						&& tar xzf sccache-v0.2.15-x86_64-unknown-linux-musl.tar.gz \
						&& mv sccache-v0.2.15-x86_64-unknown-linux-musl/sccache /usr/local/bin/sccache \
						&& chmod +x /usr/local/bin/sccache

					WORKDIR /usr/rivet
					
					COPY . .
					COPY {build_script_path} build_script.sh

					# Build and copy all binaries from target directory into an empty image (it is not
					# included in the output because of cache mount)
					RUN \
						--mount=type=cache,target=/usr/local/cargo/git \
						--mount=type=cache,target=/usr/local/cargo/registry \
						--mount=type=cache,target=/usr/rivet/target \
						chmod +x ./build_script.sh && sh -c ./build_script.sh && mkdir /usr/bin/rivet && find target/{optimization} -maxdepth 1 -type f ! -name "*.*" -exec mv {{}} /usr/bin/rivet/ \;
					
					# Create an empty image and copy binaries + test outputs to it (this is to minimize the
					# size of the image)
					FROM scratch
					COPY --from=rust /usr/bin/rivet/ /
					"#,
					build_script_path = build_script_path_relative.display(),
				),
			)
			.await?;

			// Check if we need to include default builds in the build context
			let dockerignore_path = gen_path.join("Dockerfile.build.dockerignore");
			fs::write(&dockerignore_path, DOCKERIGNORE.to_string()).await?;

			// Build image
			let mut cmd = Command::new("docker");
			cmd.env("DOCKER_BUILDKIT", "1");
			cmd.current_dir(ctx.path());
			cmd.arg("build");
			cmd.arg("--progress").arg("plain");
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
				let image_tag = format!("{push_repo}{bin}:{source_hash}");
				let dockerfile_path = gen_path.join(format!("Dockerfile.{bin}"));
				fs::write(
					&dockerfile_path,
					formatdoc!(
						r#"
						FROM {build_image_tag} AS build

						FROM debian:12.1-slim AS run

						# - Update ca-certificates
						# - Install curl for health checks
						# - Install database clients (Redis, Postgres, ClickHouse, go-migrate)
						RUN DEBIAN_FRONTEND=noninteractive apt-get update -y && \
							apt-get install -y --no-install-recommends ca-certificates openssl curl redis-tools postgresql-client clickhouse-client && \
							curl -L https://github.com/golang-migrate/migrate/releases/download/v4.18.1/migrate.linux-amd64.tar.gz | tar xvz && \
							mv migrate /usr/local/bin/migrate

						# Copy supporting scripts
						COPY {svc_scripts_path}/health_check.sh {svc_scripts_path}/install_ca.sh /usr/bin/
						RUN chmod +x /usr/bin/health_check.sh /usr/bin/install_ca.sh

						# Copy final binary
						COPY --from=build {bin} /usr/bin/{bin}
						"#,
						svc_scripts_path = svc_scripts_path_relative.display(),
					),
				)
				.await?;

				// Build image
				let mut cmd = Command::new("docker");
				cmd.env("DOCKER_BUILDKIT", "1");
				cmd.current_dir(ctx.path());
				cmd.arg("build");
				cmd.arg("--progress").arg("plain");
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

	Ok(())
}

pub const TEST_IMAGE_NAME: &str = "test";

pub struct BuildTestOpts<'a, T: AsRef<str>> {
	pub build_calls: Vec<BuildTestCall<'a, T>>,
	/// Builds for release mode.
	pub release: bool,
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
	let jobs_flag = if let Some(jobs) = opts.jobs {
		format!("--jobs {jobs}")
	} else {
		String::new()
	};

	let build_calls =
		opts.build_calls
			.iter()
			.map(|build_call| {
				let path = build_call.path.display();
				let package_flags = build_call
					.packages
					.iter()
					.map(|x| format!("--package {}", x.as_ref()))
					.collect::<Vec<_>>()
					.join(" ");
				// Generate a name from the build call path
				let name = build_call.path.iter()
					.map(|s| s.to_string_lossy())
					.collect::<Vec<_>>()
					.join("-");

				format!("(cd {path} && cargo test --no-run --message-format=json-render-diagnostics {jobs_flag} {package_flags}) > gen/tests/{name}.out")
			})
			.collect::<Vec<_>>()
			.join("\n");

	let build_script_path = ctx.gen_path().join("test_build_script.sh");
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
		export CARGO_TERM_COLOR=always

		{build_calls}
		"#,
	);

	// Write build script to file
	fs::write(&build_script_path, build_script).await?;

	let mut test_binaries = vec![];

	// Execute build command
	let temp_container_name = if ctx.build_svcs_locally() {
		// Create directory for test outputs
		fs::create_dir_all(ctx.gen_path().join("tests")).await?;

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

		None
	} else {
		let optimization = if opts.release { "release" } else { "debug" };
		// Get repo to push to
		let (push_repo, _) = ctx.docker_repos().await;
		let source_hash = ctx.source_hash();

		// Create directory for docker files
		let gen_path = ctx.gen_path().join("docker");
		fs::create_dir_all(&gen_path).await?;

		let image_tag = format!("{push_repo}{TEST_IMAGE_NAME}:{source_hash}");
		let dockerfile_path = gen_path.join(format!("Dockerfile.test_build"));

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

		// See above `build` fn for more info
		fs::write(
			&dockerfile_path,
			formatdoc!(
				r#"
				# syntax=docker/dockerfile:1.2

				FROM rust:1.81.0-slim AS build

				RUN apt-get update && apt-get install -y protobuf-compiler pkg-config libssl-dev g++ git

				RUN apt-get install --yes libpq-dev wget
				RUN wget https://github.com/mozilla/sccache/releases/download/v0.2.15/sccache-v0.2.15-x86_64-unknown-linux-musl.tar.gz \
					&& tar xzf sccache-v0.2.15-x86_64-unknown-linux-musl.tar.gz \
					&& mv sccache-v0.2.15-x86_64-unknown-linux-musl/sccache /usr/local/bin/sccache \
					&& chmod +x /usr/local/bin/sccache

				WORKDIR /usr/rivet
				
				COPY . .
				COPY {build_script_path} build_script.sh

				# TODO: Only copy test binaries (currently copies all binaries in target/opt/deps)
				# Build and copy all binaries from target directory into an empty image (it is not
				# included in the output because of cache mount)
				RUN chmod +x ./build_script.sh
				RUN mkdir -p gen/tests
				RUN \
					--mount=type=cache,target=/usr/local/cargo/git \
					--mount=type=cache,target=/usr/local/cargo/registry \
					--mount=type=cache,target=/usr/rivet/target \
					sh -c ./build_script.sh && mkdir /usr/bin/rivet && find target/{optimization}/deps -maxdepth 1 -type f ! -name "*.*" -exec mv {{}} /usr/bin/rivet/ \;
				
				FROM debian:12.1-slim AS run

				# Update ca-certificates. Install curl for health checks.
				RUN DEBIAN_FRONTEND=noninteractive apt-get update -y && apt-get install -y --no-install-recommends ca-certificates openssl curl

				# Copy supporting scripts
				COPY {svc_scripts_path}/health_check.sh {svc_scripts_path}/install_ca.sh /usr/bin/
				RUN chmod +x /usr/bin/health_check.sh /usr/bin/install_ca.sh

				# Copy generated test outputs
				COPY --from=build /usr/rivet/gen/tests/ /usr/rivet/gen/tests/

				# Copy final binaries
				COPY --from=build /usr/bin/rivet/ /usr/bin/rivet/
				"#,
				build_script_path = build_script_path_relative.display(),
				svc_scripts_path = svc_scripts_path_relative.display(),
			),
		)
		.await?;

		let dockerignore_path = gen_path.join("Dockerfile.test_build.dockerignore");
		fs::write(&dockerignore_path, DOCKERIGNORE).await?;

		// Build image
		let mut cmd = Command::new("docker");
		cmd.env("DOCKER_BUILDKIT", "1");
		cmd.current_dir(ctx.path());
		cmd.arg("build");
		cmd.arg("--progress").arg("plain");
		cmd.arg("-f").arg(dockerfile_path);
		cmd.arg("-t").arg(&image_tag);
		cmd.arg(".");

		let status = cmd.status().await?;
		ensure!(status.success(), "failed to run build command");

		// TODO: Find better way to copy files from image?
		let temp_container_name = Uuid::new_v4().to_string();
		let output_path = ctx.gen_path().display().to_string();

		// Create the temporary container
		let mut cmd = Command::new("docker");
		cmd.current_dir(ctx.path());
		cmd.arg("run");
		cmd.arg("--rm");
		cmd.arg("-d");
		cmd.arg("--name").arg(&temp_container_name);
		cmd.arg(&image_tag);
		cmd.arg("sleep").arg("120");

		let output = cmd.output().await?;
		ensure!(output.status.success(), "failed to run temp container");

		// Copy the files from the temporary container
		let mut cmd = Command::new("docker");
		cmd.current_dir(ctx.path());
		cmd.arg("cp")
			.arg(format!("{}:/usr/rivet/gen/tests", temp_container_name))
			.arg(output_path);

		let status = cmd.status().await?;
		ensure!(status.success(), "failed to copy files from container");

		Some(temp_container_name)
	};

	for build_call in opts.build_calls {
		let name = build_call
			.path
			.iter()
			.map(|s| s.to_string_lossy())
			.collect::<Vec<_>>()
			.join("-");
		let output_path = ctx.gen_path().join("tests").join(format!("{name}.out"));

		let stdout = fs::read_to_string(output_path).await?;

		// Parse artifacts
		let test_count_re = Regex::new(r"(?m)^(.*): test$").unwrap();
		for line in stdout.lines() {
			let v = serde_json::from_str::<serde_json::Value>(line).context("invalid json")?;
			if v["reason"] == "compiler-artifact" && v["target"]["kind"] == json!(["test"]) {
				if let Some(executable) = v["filenames"][0].as_str() {
					// Parsing the cargo package name (foo-bar) from
					// path+file:///foo/bar#foo-bar@0.0.1
					let package_id = v["package_id"].as_str().context("missing package_id")?;
					let package = if package_id.contains('@') {
						package_id
							.split_once('#')
							.context("split_once failed")?
							.1
							.split_once('@')
							.context("split_once failed")?
							.0
					} else {
						package_id
							.split_once('#')
							.context("split_once failed")?
							.0
							.rsplit_once('/')
							.context("split_once failed")?
							.1
					};

					let target = v["target"]["name"]
						.as_str()
						.context("missing target name")?;

					// Parse the test count from the binary
					let (exec_path, test_list_stdout) =
						if let Some(temp_container_name) = &temp_container_name {
							let exec_name = Path::new(executable);
							let exec_path = Path::new("/usr/bin/rivet")
								.join(exec_name.file_name().expect("no file name"));

							let test_list_args = [
								&[
									"exec".to_string(),
									temp_container_name.clone(),
									exec_path.display().to_string(),
									"--list".into(),
									"--format".into(),
									"terse".into(),
								],
								opts.test_filters,
							]
							.concat();

							(
								exec_path,
								block_in_place(|| duct::cmd("docker", &test_list_args).read())?,
							)
						} else {
							// Make path relative to project
							let relative_path = Path::new(executable)
								.strip_prefix(ctx.cargo_target_dir())
								.context(format!("path not in project: {executable}"))?;

							let test_list_args = [
								&["--list".to_string(), "--format".into(), "terse".into()],
								opts.test_filters,
							]
							.concat();

							(
								relative_path.to_path_buf(),
								block_in_place(|| duct::cmd(executable, &test_list_args).read())?,
							)
						};

					// Run a test container for every test in the binary
					for cap in test_count_re.captures_iter(&test_list_stdout) {
						let test_name = &cap[1];
						test_binaries.push(TestBinary {
							package: package.to_string(),
							target: target.to_string(),
							path: exec_path.clone(),
							test_name: test_name.to_string(),
						})
					}
				}
			}
		}
	}

	Ok(test_binaries)
}

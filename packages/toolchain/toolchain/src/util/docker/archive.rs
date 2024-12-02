use anyhow::*;
use serde::Deserialize;
use serde_json::json;
use std::io::Read;
use typed_path::{TryAsRef, UnixPath};
use uuid::Uuid;

use crate::{
	config::{self},
	util::{
		cmd::{self, shell_cmd, shell_cmd_std},
		task,
	},
};

pub async fn create_archive(
	task: task::TaskCtx,
	image_tag: &str,
	build_kind: config::build::docker::BundleKind,
	build_compression: config::build::Compression,
	allow_root: bool,
) -> Result<tempfile::TempPath> {
	task.log(format!(
		"[Archiving Image] {} {}",
		build_kind.as_ref(),
		build_compression.as_ref()
	));

	// Build archive
	let build_tar_path = match build_kind {
		config::build::docker::BundleKind::DockerImage => {
			archive_docker_image(task, &image_tag).await?
		}
		config::build::docker::BundleKind::OciBundle => {
			archive_oci_bundle(task, &image_tag, allow_root).await?
		}
	};

	// Compress archive
	let compressed_path =
		crate::util::build::compress_build(build_tar_path.as_ref(), build_compression).await?;

	Ok(compressed_path)
}

/// Save Docker image
async fn archive_docker_image(task: task::TaskCtx, image_tag: &str) -> Result<tempfile::TempPath> {
	let build_tar_path = tempfile::NamedTempFile::new()?.into_temp_path();

	let mut build_cmd = shell_cmd("docker");
	build_cmd
		.arg("save")
		.arg("--output")
		.arg(&build_tar_path)
		.arg(&image_tag);
	cmd::execute_docker_cmd(task, build_cmd, "Docker failed to save image").await?;

	// We have no way of validating that this Docker image is not running as root, so the container
	// will fail on setup at runtime if attempting to run as UID/GID 0.

	Ok(build_tar_path)
}

/// Convert the Docker image to an OCI bundle
///
/// This entire operation works by manipulating TAR files without touching the
/// host's file system in order to preserve file permissions & ownership on
/// Windows.
async fn archive_oci_bundle(
	task: task::TaskCtx,
	image_tag: &str,
	allow_root: bool,
) -> Result<tempfile::TempPath> {
	// Create OCI bundle
	let oci_bundle_tar_file = tempfile::NamedTempFile::new()?;
	let mut oci_bundle_archive = tar::Builder::new(oci_bundle_tar_file);

	// Create container and copy files to rootfs
	let image_tag_clone = image_tag.to_owned();
	let (
		mut oci_bundle_archive,
		CopyContainerToRootFsOutput {
			passwd_file,
			group_file,
		},
	) = tokio::task::spawn_blocking(move || -> Result<_> {
		let output = copy_container_to_rootfs(&mut oci_bundle_archive, &image_tag_clone)?;
		Result::Ok((oci_bundle_archive, output))
	})
	.await??;

	// Convert Docker image to OCI bundle
	//
	// See umoci implementation: https://github.com/opencontainers/umoci/blob/312b2db3028f823443d6a74d86b05f65701b0d0e/oci/config/convert/runtime.go#L183
	{
		#[derive(Debug, Deserialize)]
		#[serde(rename_all = "PascalCase")]
		struct DockerImage {
			config: DockerImageConfig,
		}

		#[derive(Debug, Deserialize)]
		#[serde(rename_all = "PascalCase")]
		struct DockerImageConfig {
			cmd: Option<Vec<String>>,
			entrypoint: Option<Vec<String>>,
			env: Vec<String>,
			user: String,
			#[serde(default)]
			working_dir: String,
		}

		// Inspect image
		let mut inspect_cmd = shell_cmd("docker");
		inspect_cmd.arg("image").arg("inspect").arg(&image_tag);
		let inspect_output = cmd::execute_docker_cmd_silent_fallible(inspect_cmd).await?;
		let image = serde_json::from_slice::<Vec<DockerImage>>(&inspect_output.stdout)?;
		let image = image.into_iter().next().context("no image")?;

		// Read config
		let mut config = serde_json::from_slice::<serde_json::Value>(include_bytes!(
			"../../../static/oci-bundle-config.base.json"
		))?;

		// WORKDIR
		//
		// https://github.com/opencontainers/umoci/blob/312b2db3028f823443d6a74d86b05f65701b0d0e/oci/config/convert/runtime.go#L144
		if image.config.working_dir != "" {
			config["process"]["cwd"] = json!(image.config.working_dir);
		} else {
			config["process"]["cwd"] = json!("/");
		}

		// ENV
		//
		// https://github.com/opencontainers/umoci/blob/312b2db3028f823443d6a74d86b05f65701b0d0e/oci/config/convert/runtime.go#L149
		config["process"]["env"] = json!(image.config.env);

		// ENTRYPOINT + CMD
		//
		// https://github.com/opencontainers/umoci/blob/312b2db3028f823443d6a74d86b05f65701b0d0e/oci/config/convert/runtime.go#L157
		let args = std::iter::empty::<String>()
			.chain(image.config.entrypoint.into_iter().flatten())
			.chain(image.config.cmd.into_iter().flatten())
			.collect::<Vec<_>>();
		config["process"]["args"] = json!(args);

		// USER
		//
		// https://github.com/opencontainers/umoci/blob/312b2db3028f823443d6a74d86b05f65701b0d0e/oci/config/convert/runtime.go#L174
		//
		// Moby passwd parser: https://github.com/moby/sys/blob/c0711cde08c8fa33857a2c28721659267f49b5e2/user/user.go
		//
		// If you're you're the guy at Docker who decided to reimplement passwd in Go for funzies, please reconsider next time.
		{
			// If Docker did not specify a user
			let no_dockerfile_user = image.config.user.is_empty();

			// Parse user and group from the Dockerfile
			let (dockerfile_user, dockerfile_group) =
				if let Some((u, g)) = image.config.user.split_once(":") {
					(u, Some(g))
				} else {
					(image.config.user.as_str(), None)
				};

			// Attempt to parse the user and group as integers. If the uid is
			// not specified explicitly, the uid and associated gid will be
			// looked up in the passwd & groups file below.
			let dockerfile_user_int = dockerfile_user.parse::<u32>().ok();
			let dockerfile_group_int = dockerfile_group.and_then(|x| x.parse::<u32>().ok());

			// Parse passwd file and find user
			let users = super::users::read_passwd_file(
				passwd_file.as_ref().map(|x| x.as_str()).unwrap_or_default(),
			)?;
			let exec_user = users.iter().find(|x| {
				dockerfile_user_int.map_or(false, |uid| x.uid == uid) || x.name == dockerfile_user
			});

			// Determine uid
			let uid = if no_dockerfile_user {
				0
			} else if let Some(exec_user) = exec_user {
				exec_user.uid
			} else if let Some(uid) = dockerfile_user_int {
				uid
			} else {
				task.log(format!("Warning: Cannot determine uid {} not in passwd file. Please specify a raw uid like `USER 1000:1000`.", image.config.user));
				0
			};

			// Parse group file and find group
			let groups = super::users::read_group_file(
				group_file.as_ref().map(|x| x.as_str()).unwrap_or_default(),
			)?;
			let exec_group = groups.iter().find(|x| {
				if let Some(group) = dockerfile_group {
					// Group specified explicitly in Dockerfile

					if let Some(gid) = dockerfile_group_int {
						return x.gid == gid;
					} else {
						x.name == group
					}
				} else if let Some(exec_user) = &exec_user {
					// Group not specified, assume by the gid in the passwd or if the user is in the user list
					exec_user.gid == x.gid || x.user_list.contains(&exec_user.name)
				} else {
					false
				}
			});

			// Determine gid
			let gid = if no_dockerfile_user {
				0
			} else if let Some(exec_group) = exec_group {
				exec_group.gid
			} else if let Some(gid) = dockerfile_group_int {
				gid
			} else {
				task.log(format!("Warning: Cannot determine gid. {} not in group file, please specify a raw uid & gid like `USER 1000:1000`", image.config.user));

				0
			};

			// Validate not running as root
			//
			// See Kubernetes implementation https://github.com/kubernetes/kubernetes/blob/cea1d4e20b4a7886d8ff65f34c6d4f95efcb4742/pkg/kubelet/kuberuntime/security_context_others.go#L44C4-L44C4
			if !allow_root {
				if uid == 0 {
					bail!("cannot run Docker container as root user (i.e. uid 0) for security. see https://rivet.gg/docs/dynamic-servers/concepts/docker-root-user")
				}
				if gid == 0 {
					bail!("cannot run Docker container as root group (i.e. gid 0) for security. see https://rivet.gg/docs/dynamic-servers/concepts/docker-root-user")
				}
			}

			// Specify user
			config["process"]["user"]["uid"] = json!(uid);
			config["process"]["user"]["gid"] = json!(gid);

			// Add home if needed
			if let Some(home) = exec_user.as_ref().map(|x| x.home.as_str()) {
				if !home.is_empty() {
					config["process"]["env"]
						.as_array_mut()
						.unwrap()
						.push(json!(format!("HOME={home}")));
				}
			}
		}

		// Write config.json
		{
			let config_buf = serde_json::to_vec_pretty(&config)?;

			let mut header = tar::Header::new_gnu();
			header.set_path("config.json")?;
			header.set_size(config_buf.len() as u64);
			header.set_cksum();

			oci_bundle_archive.append(&header, config_buf.as_slice())?;
		}
	}

	// Finish archive
	let oci_bundle_tar_file = oci_bundle_archive.into_inner()?;

	Ok(oci_bundle_tar_file.into_temp_path())
}

struct CopyContainerToRootFsOutput {
	passwd_file: Option<String>,
	group_file: Option<String>,
}

/// Copies files out of the container in to a TAR builder for the OCI bundle
/// under `rootfs`.
fn copy_container_to_rootfs(
	oci_bundle_archive: &mut tar::Builder<tempfile::NamedTempFile>,
	image_tag: &str,
) -> Result<CopyContainerToRootFsOutput> {
	let container_name = format!("rivet-game-{}", Uuid::new_v4());

	// Files that we need to scrape from the bundle for metadata
	let mut passwd_file = Option::<String>::None;
	let mut group_file = Option::<String>::None;

	// Create container for the image so we can copy files out of it
	let create_cmd = shell_cmd_std("docker")
		.arg("container")
		.arg("create")
		.arg("--name")
		.arg(&container_name)
		.arg(&image_tag)
		.output()?;
	ensure!(
		create_cmd.status.success(),
		"failed to create container:\n{}",
		String::from_utf8_lossy(create_cmd.stderr.as_slice())
	);

	// Copy files out of container as a TAR stream
	let mut cp_cmd = shell_cmd_std("docker");
	cp_cmd
		.arg("container")
		.arg("cp")
		.arg("--archive")
		.arg(format!("{container_name}:/"))
		.arg("-")
		.stdout(std::process::Stdio::piped());
	let mut cp_child = cp_cmd.spawn()?;

	let root_path = UnixPath::new("rootfs");

	// Read TAR stream and copy in to OCI bundle
	let rootfs_stream = cp_child.stdout.take().context("cp_child.stdout.take()")?;
	let mut archive = tar::Archive::new(rootfs_stream);
	for entry in archive.entries()? {
		let mut entry = entry?;
		let mut header = entry.header().clone();

		// Update headers to point to rootfs
		//
		// Don't use `header.set_path` because `builder.append_data` will handle long path names for
		// us
		let old_path_bytes = header.path_bytes();
		let old_path = UnixPath::new(old_path_bytes.as_ref() as &[u8]);
		let old_path_relative = old_path.strip_prefix("/")?;
		let new_path = root_path.join(old_path_relative);
		let new_path_std = new_path.try_as_ref().context(format!(
			"failed to convert unix path to os path: {new_path:?}"
		))?;

		// Read any files that we need to scrape for metadata
		//
		// We return the data we read to memory so we can write it to the OCI
		// bundle archive. If we tried to run `read_to_string` on the entry
		// again, it would return nothing.
		let read_data = match old_path
			.to_str()
			.context("failed to match old_path as str")?
		{
			"/etc/passwd" => {
				let mut s = String::new();
				entry.read_to_string(&mut s)?;
				passwd_file = Some(s.clone());
				Some(s)
			}
			"/etc/group" => {
				let mut s = String::new();
				entry.read_to_string(&mut s)?;
				group_file = Some(s.clone());
				Some(s)
			}
			_ => None,
		};

		// Update hard links to point to new rootfs.
		//
		// Hard links are resolved when TAR extracts the archive, so these need to point to the new
		// location within the archive.
		//
		// Soft links don't need to be updated (specifically for absolute paths). This is because
		// runc will chroot in to the rootfs and resolve the soft link once the container is running from there. For example,
		// an absolute symlink to `/foo/bar` within runc (i.e. chroot) will
		// resolve to `./rootfs/bin/foo`.
		if header.entry_type().is_hard_link() {
			if let Some(old_link) = header.link_name_bytes() {
				let old_link = UnixPath::new(old_link.as_ref() as &[u8]);

				if old_link.is_absolute() {
					let old_link_relative = old_link.strip_prefix("/")?;
					let new_link = root_path.join(old_link_relative);
					let new_link_std = new_link.try_as_ref().context(format!(
						"failed to convert unix path to os path: {new_link:?}"
					))?;

					// We have to use `append_link` because it allows us to pass long
					// paths.
					oci_bundle_archive.append_link(&mut header, new_path_std, new_link_std)?;

					continue;
				}
			}
		}

		// Write entry data to TAR
		//
		// We have to use `append_data` specifically because it allows us to pass long paths.
		if let Some(read_data) = read_data {
			oci_bundle_archive.append_data(
				&mut header,
				new_path_std,
				std::io::Cursor::new(read_data),
			)?;
		} else {
			oci_bundle_archive.append_data(&mut header, new_path_std, &mut entry)?;
		}
	}

	// Wait for cp to finish
	let cp_output = cp_child.wait_with_output()?;
	ensure!(
		cp_output.status.success(),
		"failed to copy files out of container:\n{}",
		String::from_utf8_lossy(cp_output.stderr.as_slice())
	);

	// Clean up container
	let rm_cmd = shell_cmd_std("docker")
		.arg("container")
		.arg("rm")
		.arg("--force")
		.arg(&container_name)
		.output()?;
	ensure!(
		rm_cmd.status.success(),
		"failed to remove container:\n{}",
		String::from_utf8_lossy(rm_cmd.stderr.as_slice())
	);

	Result::Ok(CopyContainerToRootFsOutput {
		passwd_file,
		group_file,
	})
}

use anyhow::*;
use console::style;
use futures_util::stream::StreamExt;
use rivet_api::models;
use std::{
	path::{Component, Path, PathBuf},
	time::{Duration, Instant},
};
use tokio::{
	fs::File,
	io::{AsyncReadExt, AsyncSeekExt},
};
use tokio_util::io::ReaderStream;

use crate::util::{task, term};

/// Prepared file that will be uploaded to S3.
#[derive(Clone)]
pub struct UploadFile {
	pub absolute_path: PathBuf,
	pub prepared: models::UploadPrepareFile,
}

pub fn format_file_size(bytes: u64) -> Result<String> {
	use humansize::FileSize;

	let size = format!(
		"{}",
		bytes
			.file_size(humansize::file_size_opts::BINARY)
			.ok()
			.context("bytes.file_size(...)")?
	);
	Ok(size)
}

/// Lists all files in a directory and returns the data required to upload them.
pub fn prepare_upload_dir(base_path: &Path) -> Result<Vec<UploadFile>> {
	let mut files = Vec::<UploadFile>::new();

	// Walk files while respecting .rivet-cdn-ignore
	let walk = ignore::WalkBuilder::new(base_path)
		.standard_filters(false)
		.add_custom_ignore_filename(".rivet-cdn-ignore")
		.parents(true)
		.build();
	for entry in walk {
		let entry = entry?;
		let file_meta = entry.metadata()?;

		if file_meta.is_file() {
			let file_path = entry.path();

			files.push(prepare_upload_file(
				file_path,
				file_path.strip_prefix(base_path)?,
				file_meta,
			)?);
		}
	}

	Ok(files)
}

pub fn prepare_upload_file<P: AsRef<Path>, Q: AsRef<Path>>(
	absolute_path: P,
	upload_path: Q,
	metadata: std::fs::Metadata,
) -> Result<UploadFile> {
	let absolute_path = absolute_path.as_ref();

	// Convert path to Unix-style string
	let path_str = upload_path
		.as_ref()
		.components()
		.filter_map(|c| match c {
			Component::Normal(name) => name.to_str().map(str::to_string),
			_ => None,
		})
		.collect::<Vec<String>>()
		.join("/");

	// Attempt to guess the MIME type
	let content_type = mime_guess::from_path(&absolute_path)
		.first_raw()
		.map(str::to_string);

	Ok(UploadFile {
		absolute_path: absolute_path.to_path_buf(),
		prepared: models::UploadPrepareFile {
			path: path_str,
			content_type,
			content_length: metadata.len() as i64,
		},
	})
}

/// Uploads a file to a given URL.
pub async fn upload_file(
	task: task::TaskCtx,
	reqwest_client: &reqwest::Client,
	presigned_req: &models::UploadPresignedRequest,
	file_path: impl AsRef<Path>,
	content_type: Option<impl ToString>,
	main_pb: term::EitherProgressBar,
) -> Result<()> {
	let content_type = content_type.map(|x| x.to_string());
	let path = presigned_req.path.clone();

	let is_tty = console::Term::buffered_stderr().is_term();
	let mut pb_added = false;
	let pb = match &main_pb {
		term::EitherProgressBar::Single(pb) => pb.clone(),
		term::EitherProgressBar::Multi(_) => term::progress_bar(task.clone()),
	};

	// Try the upload multiple times since DigitalOcean spaces is incredibly
	// buggy and spotty internet connections may cause issues. This is
	// especially important since we have files that we need to batch upload, so
	// one failing request is bad.
	let mut attempts = 0;
	let (upload_time, total_size) = 'upload: loop {
		let pb = pb.clone();

		// Read file
		let mut file = File::open(file_path.as_ref()).await.with_context(|| {
			anyhow!(
				"failed to open file to upload: {}",
				file_path.as_ref().display()
			)
		})?;
		let file_meta = file.metadata().await?;
		let file_len = file_meta.len();

		let total_size = presigned_req.content_length as u64;
		let is_multipart = total_size != file_len;

		let msg = if is_multipart {
			format!("{path} {}", style("[CHUNK]").dim().blue(),)
		} else {
			path.clone()
		};

		// Add progress bar
		match &main_pb {
			term::EitherProgressBar::Single(_) => {}
			term::EitherProgressBar::Multi(mpb) => {
				pb.reset();
				pb.set_message(msg);
				pb.set_length(total_size);

				// Hack to fix weird bug with `MultiProgress` where it renders an empty progress bar and leaves
				// it there
				if !pb_added {
					// pb.set_draw_target(term::get_pb_draw_target(task.clone()));
					mpb.add(pb.clone());

					pb_added = true;

					if !is_tty {
						task.log(format!(
							"Uploading {path} ({})",
							format_file_size(total_size)?
						));
					}
				}
			}
		}

		// Create a reader for the slice of the file we need to read
		file.seek(tokio::io::SeekFrom::Start(presigned_req.byte_offset as u64))
			.await?;
		let handle = file.take(presigned_req.content_length as u64);

		// Default buffer size is optimized for memory usage. Increase buffer for perf.
		let mut reader_stream = ReaderStream::with_capacity(handle, 1024 * 1024);

		let start = Instant::now();

		// Process the stream with upload progress
		let pb2 = pb.clone();
		let async_stream = async_stream::stream! {
			while let Some(chunk) = reader_stream.next().await {
				if let Result::Ok(chunk) = &chunk {
					pb2.inc(chunk.len() as u64);
				}

				yield chunk;
			}
		};

		let body = reqwest::Body::wrap_stream(async_stream);

		// Upload file
		let mut req = reqwest_client
			.put(&presigned_req.url)
			.header("content-length", presigned_req.content_length);
		if let Some(content_type) = &content_type {
			req = req.header("content-type", content_type.to_string());
		}
		let res = req.body(body).send().await?;
		if res.status().is_success() {
			let upload_time = start.elapsed();
			break 'upload (upload_time, total_size);
		} else {
			if attempts > 4 {
				let response_status = res.status();
				let response_text = res.text().await?;
				bail!(
					"failed to upload file: {}\n{:?}",
					response_status,
					response_text
				);
			} else {
				attempts += 1;

				let status = res.status();
				let body_text = res.text().await.context("res.text()")?;

				pb.set_style(term::pb_style_error());
				pb.set_message(format!(
					"{}{}{} {path} {retry_and_body}",
					style("[").bold().red(),
					style(status).bold().red(),
					style("]").bold().red(),
					path = style(&path).red(),
					retry_and_body =
						style(format!("will retry (attempt #{attempts}): {body_text:?}"))
							.dim()
							.red(),
				));

				if !is_tty {
					task.log(
						"Error uploading {path} [{status}] (attempt #{attempts}): {body_text:?}",
					);
				}

				tokio::time::sleep(Duration::from_secs(5)).await;
				continue 'upload;
			}
		}
	};

	match &main_pb {
		term::EitherProgressBar::Single(pb) => {
			pb.set_message(format!("Uploaded {path}"));
		}
		term::EitherProgressBar::Multi(_) => {
			pb.set_position(total_size);
			pb.finish();
		}
	}

	if !is_tty {
		task.log(format!(
			"Finished uploading {path} ({:.3}s)",
			upload_time.as_secs_f64()
		));
	}

	Ok(())
}

use std::path::{Path, PathBuf};

use anyhow::*;
use futures_util::StreamExt;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use url::Url;

pub async fn download_file(url: &str, file_path: &Path) -> Result<()> {
	// Fix tokio/anyhow macro bug
	use std::result::Result::{Err, Ok};

	// Create file and start request
	let (mut file, response) = tokio::try_join!(
		async {
			File::create(file_path)
				.await
				.map_err(Into::<anyhow::Error>::into)
		},
		async { reqwest::get(url).await.map_err(Into::<anyhow::Error>::into) }
	)?;

	let mut stream = response.bytes_stream();

	// Write from stream to file
	while let Some(chunk) = stream.next().await {
		file.write_all(&chunk?).await?;
	}

	anyhow::Ok(())
}

// Get `UUID/job-runner` from URL (HOST/s3-cache/aws/BUCKET/job-runner/UUID/job-runner)
pub fn get_s3_path_stub(url: &Url, with_uuid: bool) -> Result<PathBuf> {
	let path_segments = url.path_segments().context("bad container runner url")?;
	let path_stub = path_segments
		.rev()
		.take(if with_uuid { 2 } else { 1 })
		.collect::<Vec<_>>()
		.into_iter()
		.rev()
		.collect::<PathBuf>();

	Ok(path_stub)
}

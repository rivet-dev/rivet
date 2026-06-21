//! Filesystem actions. Each helper takes `&AgentOs` plus typed args
//! and delegates to the matching upstream `AgentOs::*` method. DTOs
//! used by batch operations live here too so the dispatcher arms can
//! deserialize/serialize directly without re-declaring shapes.

use agent_os_client::{
	AgentOs, BatchReadResult, BatchWriteEntry, BatchWriteResult, DeleteOptions, DirEntry,
	FileContent, MkdirOptions, ReaddirRecursiveOptions, VirtualStat,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// `readFile(path)` — port of [`AgentOs::read_file`].
pub async fn read_file(vm: &AgentOs, path: &str) -> Result<Vec<u8>> {
	vm.read_file(path)
		.await
		.inspect_err(|error| tracing::error!(?error, path, "read_file failed"))
}

/// `writeFile(path, contents)` — port of [`AgentOs::write_file`].
pub async fn write_file(vm: &AgentOs, path: &str, contents: Vec<u8>) -> Result<()> {
	vm.write_file(path, FileContent::Bytes(contents))
		.await
		.inspect_err(|error| tracing::error!(?error, path, "write_file failed"))
}

/// `stat(path)` — port of [`AgentOs::stat`]. Returns the [`VirtualStat`]
/// structure directly; the rivetkit encoder handles cross-encoding
/// translation (bare / cbor / json) at the framework layer.
pub async fn stat(vm: &AgentOs, path: &str) -> Result<VirtualStat> {
	vm.stat(path).await
}

/// `mkdir(path)` — port of [`AgentOs::mkdir`]. Always recursive so the
/// JS shim's "create parent dirs if needed" expectation holds; the
/// driver tests rely on this.
pub async fn mkdir(vm: &AgentOs, path: &str) -> Result<()> {
	vm.mkdir(path, MkdirOptions { recursive: true }).await
}

/// `readdir(path)` — port of [`AgentOs::readdir`]. Returns the
/// (unsorted) child names, including `.` and `..`. Sorting / filtering
/// is up to the caller.
pub async fn readdir(vm: &AgentOs, path: &str) -> Result<Vec<String>> {
	vm.readdir(path).await
}

/// `exists(path)` — port of [`AgentOs::exists`].
pub async fn exists(vm: &AgentOs, path: &str) -> Result<bool> {
	vm.exists(path).await
}

/// `move(from, to)` — port of [`AgentOs::move_path`]. Named `move_path`
/// in Rust because `move` is a keyword.
pub async fn move_path(vm: &AgentOs, from: &str, to: &str) -> Result<()> {
	vm.move_path(from, to).await
}

/// Options for `deleteFile`. TS sends `{ recursive?: boolean }`.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteOptionsArg {
	#[serde(default)]
	pub recursive: bool,
}

/// `deleteFile(path, options?)` — port of [`AgentOs::delete`]. Honors the
/// `recursive` option so directory deletes match JS semantics.
pub async fn delete_file(vm: &AgentOs, path: &str, recursive: bool) -> Result<()> {
	vm.delete(path, DeleteOptions { recursive }).await
}

/// `writeFiles(entries)` — port of [`AgentOs::write_files`]. Per-entry
/// failures are reported in the [`BatchWriteResultDto`]'s `success` /
/// `error` fields rather than as a top-level error.
pub async fn write_files(
	vm: &AgentOs,
	entries: Vec<WriteFilesEntryArg>,
) -> Vec<BatchWriteResultDto> {
	let entries: Vec<BatchWriteEntry> = entries
		.into_iter()
		.map(|entry| BatchWriteEntry {
			path: entry.path,
			content: FileContent::Bytes(entry.content.into_bytes()),
		})
		.collect();
	vm.write_files(entries)
		.await
		.into_iter()
		.map(BatchWriteResultDto::from)
		.collect()
}

/// `readFiles(paths)` — port of [`AgentOs::read_files`]. Per-entry
/// failures are reported as `content: None` plus an error string.
pub async fn read_files(vm: &AgentOs, paths: Vec<String>) -> Vec<BatchReadResultDto> {
	vm.read_files(paths)
		.await
		.into_iter()
		.map(BatchReadResultDto::from)
		.collect()
}

/// `readdirRecursive(path)` — port of [`AgentOs::readdir_recursive`].
/// Returns every reachable entry with its type and size. Unbounded
/// depth; the JS shim passes no max-depth in the driver tests so this
/// arm defaults to `ReaddirRecursiveOptions::default()`.
pub async fn readdir_recursive(vm: &AgentOs, path: &str) -> Result<Vec<DirEntry>> {
	vm.readdir_recursive(path, ReaddirRecursiveOptions::default())
		.await
}

// ---------------------------------------------------------------------------
// Action argument / reply DTOs
// ---------------------------------------------------------------------------

/// Accept either a CBOR text string, a CBOR byte string (via `ByteBuf`), or
/// the `["$Uint8Array", base64]` wrapper that TS encoders emit when the
/// outer codec is JSON-compatible. Used by `writeFile` and `writeFiles`.
#[derive(Deserialize)]
#[serde(untagged)]
pub enum WriteFileContent {
	String(String),
	Bytes(serde_bytes::ByteBuf),
	Wrapped(JsonCompatUint8Array),
}

impl WriteFileContent {
	pub fn into_bytes(self) -> Vec<u8> {
		match self {
			Self::String(s) => s.into_bytes(),
			Self::Bytes(b) => b.into_vec(),
			Self::Wrapped(w) => w.bytes,
		}
	}
}

/// Deserializer for the `["$Uint8Array", base64]` envelope. Part of
/// [`WriteFileContent`]'s untagged enum so the same arms accept wrapped
/// bytes from the JSON encoder path.
pub struct JsonCompatUint8Array {
	bytes: Vec<u8>,
}

impl<'de> Deserialize<'de> for JsonCompatUint8Array {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
		let (tag, base64): (String, String) = Deserialize::deserialize(deserializer)?;
		if tag != "$Uint8Array" {
			return Err(serde::de::Error::custom(format!(
				"expected $Uint8Array wrapper, got {tag}"
			)));
		}
		let bytes = BASE64
			.decode(&base64)
			.map_err(|error| serde::de::Error::custom(format!("base64 decode: {error}")))?;
		Ok(Self { bytes })
	}
}

/// Argument entry for `writeFiles`. TS sends `[{path, content}, ...]`
/// where `content` follows the same coercion rules as `writeFile`.
#[derive(Deserialize)]
pub struct WriteFilesEntryArg {
	pub path: String,
	pub content: WriteFileContent,
}

/// Reply entry for `writeFiles`. Mirrors `BatchWriteResult` in a
/// serializable form. `error` is `None` on success.
#[derive(Serialize)]
pub struct BatchWriteResultDto {
	pub path: String,
	pub success: bool,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub error: Option<String>,
}

impl From<BatchWriteResult> for BatchWriteResultDto {
	fn from(value: BatchWriteResult) -> Self {
		Self {
			path: value.path,
			success: value.success,
			error: value.error,
		}
	}
}

/// Reply entry for `readFiles`. `content` is wrapped via `serde_bytes`
/// so the `JsonCompatAdapter` re-wraps it as `["$Uint8Array", base64]`
/// for JSON encoders. `None` content + `Some(error)` indicates that the
/// specific file failed without aborting the whole batch.
#[derive(Serialize)]
pub struct BatchReadResultDto {
	pub path: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub content: Option<serde_bytes::ByteBuf>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub error: Option<String>,
}

impl From<BatchReadResult> for BatchReadResultDto {
	fn from(value: BatchReadResult) -> Self {
		Self {
			path: value.path,
			content: value.content.map(serde_bytes::ByteBuf::from),
			error: value.error,
		}
	}
}

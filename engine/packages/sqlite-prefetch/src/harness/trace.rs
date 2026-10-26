//! The log parser: `sqlite_prefetch::trace` events from a process log, in the plain `tracing`
//! format or as JSON lines, back into the client events the testing VFS replays.
//!
//! A recorded trace carries page numbers only, never page bytes, so its replay serves zeroed
//! pages: it scores the window and the recent pages, not the parts of the library that read
//! page contents. Synthetic workloads run real SQLite and score everything.
use std::collections::BTreeMap;
use std::io::Read;
use std::sync::Arc;

use anyhow::{Context, Result};

use super::server::{Server, Strategy};
use super::store::MemStore;
use super::vfs::{Access, Client, ClientCounts, Event};
use crate::page::Pgno;

/// Renders events as the log lines the production VFS writes.
pub fn render(events: &[Event]) -> String {
	let mut out = String::new();
	let mut commit_id = 0u64;
	for event in events {
		match event {
			Event::Read(access) => out.push_str(&format!(
				"2026-10-25T00:00:00.000000Z TRACE sqlite_prefetch::trace: access event=\"access\" pgno={} op=\"read\" hit={} request_id={}\n",
				access.pgno, access.hit, access.request_id
			)),
			Event::Commit { pages, size_pages } => {
				commit_id += 1;
				for pgno in pages {
					out.push_str(&format!(
						"2026-10-25T00:00:00.000000Z TRACE sqlite_prefetch::trace: access event=\"access\" pgno={pgno} op=\"write\" hit=false request_id=c{commit_id} size_pages={size_pages}\n"
					));
				}
				out.push_str(&format!(
					"2026-10-25T00:00:00.000000Z TRACE sqlite_prefetch::trace: commit event=\"commit\" request_id=c{commit_id} size_pages={size_pages}\n"
				));
			}
			Event::Reopen => out.push_str(
				"2026-10-25T00:00:00.000000Z TRACE sqlite_prefetch::trace: open event=\"open\"\n",
			),
		}
	}
	out
}

/// Parses the trace events out of a log, ignoring every other line.
pub fn parse(text: &str) -> Result<Vec<Event>> {
	let streams: std::collections::BTreeSet<_> = text
		.lines()
		.filter_map(fields)
		.filter(|fields| {
			matches!(
				fields.get("event").map(String::as_str),
				Some("access" | "commit" | "open")
			)
		})
		.map(|fields| fields.get("stream_id").cloned())
		.collect();
	anyhow::ensure!(
		streams.len() <= 1,
		"trace contains multiple database streams; select one with parse_stream"
	);
	parse_selected(text, None)
}

/// Selects one database opening stream from interleaved production logs.
pub fn parse_stream(text: &str, stream_id: &str) -> Result<Vec<Event>> {
	parse_selected(text, Some(stream_id))
}

fn parse_selected(text: &str, stream_id: Option<&str>) -> Result<Vec<Event>> {
	let mut events = Vec::new();
	let mut commit: Option<(String, Vec<Pgno>)> = None;
	for line in text.lines() {
		let Some(fields) = fields(line) else {
			continue;
		};
		if stream_id
			.is_some_and(|stream| fields.get("stream_id").map(String::as_str) != Some(stream))
		{
			continue;
		}
		let event = fields.get("event").map(String::as_str).unwrap_or("access");
		match event {
			"access" => {
				let pgno: Pgno = fields
					.get("pgno")
					.context("access without pgno")?
					.parse()
					.context("pgno")?;
				let op = fields.get("op").map(String::as_str).unwrap_or("read");
				let request = fields.get("request_id").cloned().unwrap_or_default();
				match op {
					"write" => match &mut commit {
						Some((id, pages)) if *id == request => pages.push(pgno),
						_ => commit = Some((request, vec![pgno])),
					},
					_ => events.push(Event::Read(Access {
						pgno,
						hit: fields.get("hit").is_some_and(|hit| hit == "true"),
						request_id: request.parse().unwrap_or(0),
					})),
				}
			}
			"commit" => {
				let size_pages: u32 = fields
					.get("size_pages")
					.context("commit without size_pages")?
					.parse()
					.context("size_pages")?;
				let pages = match commit.take() {
					Some((id, pages)) => {
						anyhow::ensure!(
							fields.get("request_id") == Some(&id),
							"commit does not match its writes"
						);
						pages
					}
					None => Vec::new(),
				};
				events.push(Event::Commit { pages, size_pages });
			}
			"open" => {
				commit = None;
				events.push(Event::Reopen);
			}
			_ => {}
		}
	}
	Ok(events)
}

/// Parses a gzipped log.
pub fn parse_gz(bytes: &[u8]) -> Result<Vec<Event>> {
	let mut text = String::new();
	flate2::read::GzDecoder::new(bytes)
		.read_to_string(&mut text)
		.context("decompress trace")?;
	parse(&text)
}

/// The fields of a trace line: `key=value` pairs after the target in the plain format, or the
/// `fields` object of a JSON line.
fn fields(line: &str) -> Option<BTreeMap<String, String>> {
	let at = line.find(crate::TRACE_TARGET)?;
	let rest = &line[at + crate::TRACE_TARGET.len()..];
	let mut fields = BTreeMap::new();
	if line.trim_start().starts_with('{') {
		let json: serde_json::Value = serde_json::from_str(line).ok()?;
		if json.get("target")?.as_str()? != crate::TRACE_TARGET {
			return None;
		}
		for (key, value) in json.get("fields")?.as_object()? {
			fields.insert(
				key.clone(),
				value
					.as_str()
					.map(str::to_string)
					.unwrap_or_else(|| value.to_string()),
			);
		}
		return Some(fields);
	}
	for token in rest.split_whitespace() {
		let Some((key, value)) = token.split_once('=') else {
			continue;
		};
		fields.insert(key.to_string(), value.trim_matches('"').to_string());
	}
	Some(fields)
}

/// Replays a trace's reads and commits through a strategy, with zeroed pages.
pub fn replay(events: &[Event], strategy: Strategy) -> ClientCounts {
	let size = events
		.iter()
		.map(|event| match event {
			Event::Read(access) => access.pgno,
			Event::Commit { size_pages, .. } => *size_pages,
			Event::Reopen => 0,
		})
		.max()
		.unwrap_or(0);
	let store = Arc::new(MemStore::default());
	let zero = bytes::Bytes::from(vec![0u8; super::store::PAGE]);
	let pages: BTreeMap<Pgno, bytes::Bytes> = (1..=size).map(|pgno| (pgno, zero.clone())).collect();
	store.commit(&pages, size, &[]);
	let mut server = Server::new(strategy, store);
	server.future = events
		.iter()
		.filter_map(|event| match event {
			Event::Read(access) => Some(access.pgno),
			Event::Commit { .. } | Event::Reopen => None,
		})
		.collect();
	server.open();
	let mut client = Client::new(server);
	client.verify_bytes = false;
	for event in events {
		match event {
			Event::Read(access) => client.replay_read(access.pgno),
			Event::Commit { pages, size_pages } => client.replay_commit(pages, *size_pages),
			Event::Reopen => client.reopen(),
		}
	}
	client.drain_buffer();
	client.counts
}

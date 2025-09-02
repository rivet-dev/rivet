use std::result::Result::Ok;

use anyhow::*;
use rivet_util::Id;
use udb_util::prelude::*;

use crate::history::{
	event::{EventType, SleepState},
	location::{Coordinate, Location},
};

// Parses workflow id and location, ignores the rest
#[derive(Debug)]
pub struct PartialEventKey {
	#[allow(dead_code)]
	workflow_id: Id,
	pub location: Location,
	pub forgotten: bool,
}

impl<'de> TupleUnpack<'de> for PartialEventKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (mut input, (_, _, workflow_id, data, history_variant)) =
			<(usize, usize, Id, usize, usize)>::unpack(input, tuple_depth)?;
		if data != HISTORY {
			return Err(PackError::Message("expected HISTORY data".into()));
		}

		let mut coords = Vec::new();

		loop {
			let Ok((input2, coord)) = Coordinate::unpack(input, tuple_depth) else {
				break;
			};

			coords.push(coord);
			input = input2;
		}

		Ok((
			// Ignore rest
			&input[0..0],
			PartialEventKey {
				workflow_id,
				location: Location::from_iter(coords),
				forgotten: history_variant == FORGOTTEN,
			},
		))
	}
}

#[derive(Debug, Clone, Copy)]
pub enum HistorySubspaceVariant {
	All,
	Active,
	Forgotten,
}

#[derive(Debug, Clone, Copy)]
pub struct HistorySubspaceKey {
	workflow_id: Id,
	variant: HistorySubspaceVariant,
}

impl HistorySubspaceKey {
	pub fn new(workflow_id: Id, variant: HistorySubspaceVariant) -> Self {
		HistorySubspaceKey {
			workflow_id,
			variant,
		}
	}
}

impl TuplePack for HistorySubspaceKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let mut offset = VersionstampOffset::None { size: 0 };

		let t = (WORKFLOW, DATA, self.workflow_id, HISTORY);
		offset += t.pack(w, tuple_depth)?;

		match self.variant {
			HistorySubspaceVariant::All => {}
			HistorySubspaceVariant::Active => {
				offset += ACTIVE.pack(w, tuple_depth)?;
			}
			HistorySubspaceVariant::Forgotten => {
				offset += FORGOTTEN.pack(w, tuple_depth)?;
			}
		}

		Ok(offset)
	}
}

#[derive(Debug, Clone)]
pub struct EventHistorySubspaceKey {
	workflow_id: Id,
	location: Location,
	idx: Option<usize>,
	forgotten: bool,
}

impl EventHistorySubspaceKey {
	pub fn new(workflow_id: Id, location: Location, idx: usize, forgotten: bool) -> Self {
		EventHistorySubspaceKey {
			workflow_id,
			location,
			idx: Some(idx),
			forgotten,
		}
	}

	pub fn entire(workflow_id: Id, location: Location, forgotten: bool) -> Self {
		EventHistorySubspaceKey {
			workflow_id,
			location,
			idx: None,
			forgotten,
		}
	}
}

impl TuplePack for EventHistorySubspaceKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let mut offset = VersionstampOffset::None { size: 0 };

		let t = (
			WORKFLOW,
			DATA,
			self.workflow_id,
			HISTORY,
			if self.forgotten { FORGOTTEN } else { ACTIVE },
		);
		offset += t.pack(w, tuple_depth)?;

		for coord in &*self.location {
			offset += coord.pack(w, tuple_depth)?;
		}

		// This ensures we are only reading events under the given location and not event data at the current
		// location
		w.write_all(&[udb_util::codes::NESTED])?;
		offset += 1;

		if let Some(idx) = self.idx {
			offset += idx.pack(w, tuple_depth)?;
		}

		Ok(offset)
	}
}

#[derive(Debug)]
pub struct EventTypeKey {
	workflow_id: Id,
	location: Location,
	forgotten: bool,
}

impl EventTypeKey {
	pub fn new(workflow_id: Id, location: Location) -> Self {
		EventTypeKey {
			workflow_id,
			location,
			forgotten: false,
		}
	}
}

impl FormalKey for EventTypeKey {
	type Value = EventType;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		EventType::from_repr(usize::from_be_bytes(raw.try_into()?)).context("invalid EventType")
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok((value as usize).to_be_bytes().to_vec())
	}
}

impl TuplePack for EventTypeKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		pack_history_key(
			self.workflow_id,
			&self.location,
			w,
			tuple_depth,
			self.forgotten,
			EVENT_TYPE,
		)
	}
}

impl<'de> TupleUnpack<'de> for EventTypeKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (workflow_id, location, forgotten)) =
			unpack_history_key(input, tuple_depth, EVENT_TYPE, "EVENT_TYPE")?;

		Ok((
			input,
			EventTypeKey {
				workflow_id,
				location,
				forgotten,
			},
		))
	}
}

#[derive(Debug)]
pub struct VersionKey {
	workflow_id: Id,
	location: Location,
	forgotten: bool,
}

impl VersionKey {
	pub fn new(workflow_id: Id, location: Location) -> Self {
		VersionKey {
			workflow_id,
			location,
			forgotten: false,
		}
	}
}

impl FormalKey for VersionKey {
	type Value = usize;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(usize::from_be_bytes(raw.try_into()?))
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.to_be_bytes().to_vec())
	}
}

impl TuplePack for VersionKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		pack_history_key(
			self.workflow_id,
			&self.location,
			w,
			tuple_depth,
			self.forgotten,
			VERSION,
		)
	}
}

impl<'de> TupleUnpack<'de> for VersionKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (workflow_id, location, forgotten)) =
			unpack_history_key(input, tuple_depth, VERSION, "VERSION")?;

		Ok((
			input,
			VersionKey {
				workflow_id,
				location,
				forgotten,
			},
		))
	}
}

#[derive(Debug)]
pub struct CreateTsKey {
	workflow_id: Id,
	location: Location,
	forgotten: bool,
}

impl CreateTsKey {
	pub fn new(workflow_id: Id, location: Location) -> Self {
		CreateTsKey {
			workflow_id,
			location,
			forgotten: false,
		}
	}
}

impl FormalKey for CreateTsKey {
	/// Timestamp.
	type Value = i64;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(i64::from_be_bytes(raw.try_into()?))
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.to_be_bytes().to_vec())
	}
}

impl TuplePack for CreateTsKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		pack_history_key(
			self.workflow_id,
			&self.location,
			w,
			tuple_depth,
			self.forgotten,
			CREATE_TS,
		)
	}
}

impl<'de> TupleUnpack<'de> for CreateTsKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (workflow_id, location, forgotten)) =
			unpack_history_key(input, tuple_depth, CREATE_TS, "CREATE_TS")?;

		Ok((
			input,
			CreateTsKey {
				workflow_id,
				location,
				forgotten,
			},
		))
	}
}

#[derive(Debug)]
pub struct NameKey {
	workflow_id: Id,
	location: Location,
	forgotten: bool,
}

impl NameKey {
	pub fn new(workflow_id: Id, location: Location) -> Self {
		NameKey {
			workflow_id,
			location,
			forgotten: false,
		}
	}
}

impl FormalKey for NameKey {
	type Value = String;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		String::from_utf8(raw.to_vec()).map_err(Into::into)
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.into_bytes())
	}
}

impl TuplePack for NameKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		pack_history_key(
			self.workflow_id,
			&self.location,
			w,
			tuple_depth,
			self.forgotten,
			NAME,
		)
	}
}

impl<'de> TupleUnpack<'de> for NameKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (workflow_id, location, forgotten)) =
			unpack_history_key(input, tuple_depth, NAME, "NAME")?;

		Ok((
			input,
			NameKey {
				workflow_id,
				location,
				forgotten,
			},
		))
	}
}

#[derive(Debug)]
pub struct SignalIdKey {
	workflow_id: Id,
	location: Location,
	forgotten: bool,
}

impl SignalIdKey {
	pub fn new(workflow_id: Id, location: Location) -> Self {
		SignalIdKey {
			workflow_id,
			location,
			forgotten: false,
		}
	}
}

impl FormalKey for SignalIdKey {
	type Value = Id;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(Id::from_slice(raw)?)
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.as_bytes().to_vec())
	}
}

impl TuplePack for SignalIdKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		pack_history_key(
			self.workflow_id,
			&self.location,
			w,
			tuple_depth,
			self.forgotten,
			SIGNAL_ID,
		)
	}
}

impl<'de> TupleUnpack<'de> for SignalIdKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (workflow_id, location, forgotten)) =
			unpack_history_key(input, tuple_depth, SIGNAL_ID, "SIGNAL_ID")?;

		Ok((
			input,
			SignalIdKey {
				workflow_id,
				location,
				forgotten,
			},
		))
	}
}

#[derive(Debug)]
pub struct SubWorkflowIdKey {
	workflow_id: Id,
	location: Location,
	forgotten: bool,
}

impl SubWorkflowIdKey {
	pub fn new(workflow_id: Id, location: Location) -> Self {
		SubWorkflowIdKey {
			workflow_id,
			location,
			forgotten: false,
		}
	}
}

impl FormalKey for SubWorkflowIdKey {
	type Value = Id;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(Id::from_slice(raw)?)
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.as_bytes().to_vec())
	}
}

impl TuplePack for SubWorkflowIdKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		pack_history_key(
			self.workflow_id,
			&self.location,
			w,
			tuple_depth,
			self.forgotten,
			SUB_WORKFLOW_ID,
		)
	}
}

impl<'de> TupleUnpack<'de> for SubWorkflowIdKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (workflow_id, location, forgotten)) =
			unpack_history_key(input, tuple_depth, SUB_WORKFLOW_ID, "SUB_WORKFLOW_ID")?;

		Ok((
			input,
			SubWorkflowIdKey {
				workflow_id,
				location,
				forgotten,
			},
		))
	}
}

pub struct InputKey {
	workflow_id: Id,
	location: Location,
	forgotten: bool,
}

impl InputKey {
	pub fn new(workflow_id: Id, location: Location) -> Self {
		InputKey {
			workflow_id,
			location,
			forgotten: false,
		}
	}

	pub fn split_ref(&self, value: &serde_json::value::RawValue) -> Result<Vec<Vec<u8>>> {
		Ok(value
			.get()
			.as_bytes()
			.chunks(udb_util::CHUNK_SIZE)
			.map(|x| x.to_vec())
			.collect())
	}
}

impl FormalChunkedKey for InputKey {
	type ChunkKey = InputChunkKey;
	type Value = Box<serde_json::value::RawValue>;

	fn chunk(&self, chunk: usize) -> Self::ChunkKey {
		InputChunkKey {
			workflow_id: self.workflow_id,
			location: self.location.clone(),
			forgotten: self.forgotten,
			chunk,
		}
	}

	fn combine(&self, chunks: Vec<FdbValue>) -> Result<Self::Value> {
		serde_json::value::RawValue::from_string(String::from_utf8(
			chunks
				.iter()
				.map(|x| x.value().iter().map(|x| *x))
				.flatten()
				.collect(),
		)?)
		.map_err(Into::into)
	}

	fn split(&self, value: Self::Value) -> Result<Vec<Vec<u8>>> {
		self.split_ref(value.as_ref())
	}
}

impl TuplePack for InputKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		pack_history_key(
			self.workflow_id,
			&self.location,
			w,
			tuple_depth,
			self.forgotten,
			INPUT,
		)
	}
}

pub struct InputChunkKey {
	workflow_id: Id,
	location: Location,
	forgotten: bool,
	chunk: usize,
}

impl TuplePack for InputChunkKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		pack_history_key(
			self.workflow_id,
			&self.location,
			w,
			tuple_depth,
			self.forgotten,
			INPUT,
		)?;

		self.chunk.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for InputChunkKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (workflow_id, location, forgotten)) =
			unpack_history_key(input, tuple_depth, INPUT, "INPUT")?;

		let (input, chunk) = <usize>::unpack(input, tuple_depth)?;

		Ok((
			input,
			InputChunkKey {
				workflow_id,
				location,
				forgotten,
				chunk,
			},
		))
	}
}

pub struct OutputKey {
	workflow_id: Id,
	location: Location,
	forgotten: bool,
}

impl OutputKey {
	pub fn new(workflow_id: Id, location: Location) -> Self {
		OutputKey {
			workflow_id,
			location,
			forgotten: false,
		}
	}

	pub fn split_ref(&self, value: &serde_json::value::RawValue) -> Result<Vec<Vec<u8>>> {
		Ok(value
			.get()
			.as_bytes()
			.chunks(udb_util::CHUNK_SIZE)
			.map(|x| x.to_vec())
			.collect())
	}
}

impl FormalChunkedKey for OutputKey {
	type ChunkKey = OutputChunkKey;
	type Value = Box<serde_json::value::RawValue>;

	fn chunk(&self, chunk: usize) -> Self::ChunkKey {
		OutputChunkKey {
			workflow_id: self.workflow_id,
			location: self.location.clone(),
			forgotten: self.forgotten,
			chunk,
		}
	}

	fn combine(&self, chunks: Vec<FdbValue>) -> Result<Self::Value> {
		serde_json::value::RawValue::from_string(String::from_utf8(
			chunks
				.iter()
				.map(|x| x.value().iter().map(|x| *x))
				.flatten()
				.collect(),
		)?)
		.map_err(Into::into)
	}

	fn split(&self, value: Self::Value) -> Result<Vec<Vec<u8>>> {
		self.split_ref(value.as_ref())
	}
}

impl TuplePack for OutputKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		pack_history_key(
			self.workflow_id,
			&self.location,
			w,
			tuple_depth,
			self.forgotten,
			OUTPUT,
		)
	}
}

pub struct OutputChunkKey {
	workflow_id: Id,
	location: Location,
	forgotten: bool,
	chunk: usize,
}

impl TuplePack for OutputChunkKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		pack_history_key(
			self.workflow_id,
			&self.location,
			w,
			tuple_depth,
			self.forgotten,
			OUTPUT,
		)?;

		self.chunk.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for OutputChunkKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (workflow_id, location, forgotten)) =
			unpack_history_key(input, tuple_depth, OUTPUT, "OUTPUT")?;

		let (input, chunk) = <usize>::unpack(input, tuple_depth)?;

		Ok((
			input,
			OutputChunkKey {
				workflow_id,
				location,
				forgotten,
				chunk,
			},
		))
	}
}

#[derive(Debug)]
pub struct InputHashKey {
	workflow_id: Id,
	location: Location,
	forgotten: bool,
}

impl InputHashKey {
	pub fn new(workflow_id: Id, location: Location) -> Self {
		InputHashKey {
			workflow_id,
			location,
			forgotten: false,
		}
	}
}

impl FormalKey for InputHashKey {
	type Value = Vec<u8>;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(raw.to_vec())
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value)
	}
}

impl TuplePack for InputHashKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		pack_history_key(
			self.workflow_id,
			&self.location,
			w,
			tuple_depth,
			self.forgotten,
			INPUT_HASH,
		)
	}
}

impl<'de> TupleUnpack<'de> for InputHashKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (workflow_id, location, forgotten)) =
			unpack_history_key(input, tuple_depth, INPUT_HASH, "INPUT_HASH")?;

		Ok((
			input,
			InputHashKey {
				workflow_id,
				location,
				forgotten,
			},
		))
	}
}

#[derive(Debug)]
pub struct ErrorKey {
	workflow_id: Id,
	location: Location,
	forgotten: bool,
	pub ts: i64,
	pub error: String,
}

impl ErrorKey {
	pub fn new(workflow_id: Id, location: Location, ts: i64, error: String) -> Self {
		ErrorKey {
			workflow_id,
			location,
			forgotten: false,
			ts,
			error,
		}
	}
}

impl FormalKey for ErrorKey {
	type Value = ();

	fn deserialize(&self, _raw: &[u8]) -> Result<Self::Value> {
		Ok(())
	}

	fn serialize(&self, _value: Self::Value) -> Result<Vec<u8>> {
		Ok(Vec::new())
	}
}

impl TuplePack for ErrorKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let mut offset = VersionstampOffset::None { size: 0 };

		offset += pack_history_key(
			self.workflow_id,
			&self.location,
			w,
			tuple_depth,
			self.forgotten,
			ERROR,
		)?;

		let t = (self.ts, &self.error);
		t.pack(w, tuple_depth)?;

		Ok(offset)
	}
}

impl<'de> TupleUnpack<'de> for ErrorKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (workflow_id, location, forgotten)) =
			unpack_history_key(input, tuple_depth, ERROR, "ERROR")?;

		let (input, (ts, error)) = <(i64, String)>::unpack(input, tuple_depth)?;

		Ok((
			input,
			ErrorKey {
				workflow_id,
				location,
				forgotten,
				ts,
				error,
			},
		))
	}
}

#[derive(Debug)]
pub struct IterationKey {
	workflow_id: Id,
	location: Location,
	forgotten: bool,
}

impl IterationKey {
	pub fn new(workflow_id: Id, location: Location) -> Self {
		IterationKey {
			workflow_id,
			location,
			forgotten: false,
		}
	}
}

impl FormalKey for IterationKey {
	type Value = usize;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(usize::from_be_bytes(raw.try_into()?))
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.to_be_bytes().to_vec())
	}
}

impl TuplePack for IterationKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		pack_history_key(
			self.workflow_id,
			&self.location,
			w,
			tuple_depth,
			self.forgotten,
			ITERATION,
		)
	}
}

impl<'de> TupleUnpack<'de> for IterationKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (workflow_id, location, forgotten)) =
			unpack_history_key(input, tuple_depth, ITERATION, "ITERATION")?;

		Ok((
			input,
			IterationKey {
				workflow_id,
				location,
				forgotten,
			},
		))
	}
}

#[derive(Debug)]
pub struct DeadlineTsKey {
	workflow_id: Id,
	location: Location,
	forgotten: bool,
}

impl DeadlineTsKey {
	pub fn new(workflow_id: Id, location: Location) -> Self {
		DeadlineTsKey {
			workflow_id,
			location,
			forgotten: false,
		}
	}
}

impl FormalKey for DeadlineTsKey {
	/// Timestamp.
	type Value = i64;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(i64::from_be_bytes(raw.try_into()?))
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.to_be_bytes().to_vec())
	}
}

impl TuplePack for DeadlineTsKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		pack_history_key(
			self.workflow_id,
			&self.location,
			w,
			tuple_depth,
			self.forgotten,
			DEADLINE_TS,
		)
	}
}

impl<'de> TupleUnpack<'de> for DeadlineTsKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (workflow_id, location, forgotten)) =
			unpack_history_key(input, tuple_depth, DEADLINE_TS, "DEADLINE_TS")?;

		Ok((
			input,
			DeadlineTsKey {
				workflow_id,
				location,
				forgotten,
			},
		))
	}
}

#[derive(Debug)]
pub struct SleepStateKey {
	workflow_id: Id,
	location: Location,
	forgotten: bool,
}

impl SleepStateKey {
	pub fn new(workflow_id: Id, location: Location) -> Self {
		SleepStateKey {
			workflow_id,
			location,
			forgotten: false,
		}
	}
}

impl FormalKey for SleepStateKey {
	type Value = SleepState;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		SleepState::from_repr(usize::from_be_bytes(raw.try_into()?)).context("invalid EventType")
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok((value as usize).to_be_bytes().to_vec())
	}
}

impl TuplePack for SleepStateKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		pack_history_key(
			self.workflow_id,
			&self.location,
			w,
			tuple_depth,
			self.forgotten,
			SLEEP_STATE,
		)
	}
}

impl<'de> TupleUnpack<'de> for SleepStateKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (workflow_id, location, forgotten)) =
			unpack_history_key(input, tuple_depth, SLEEP_STATE, "SLEEP_STATE")?;

		Ok((
			input,
			SleepStateKey {
				workflow_id,
				location,
				forgotten,
			},
		))
	}
}

#[derive(Debug)]
pub struct InnerEventTypeKey {
	workflow_id: Id,
	location: Location,
	forgotten: bool,
}

impl InnerEventTypeKey {
	pub fn new(workflow_id: Id, location: Location) -> Self {
		InnerEventTypeKey {
			workflow_id,
			location,
			forgotten: false,
		}
	}
}

impl FormalKey for InnerEventTypeKey {
	type Value = EventType;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		EventType::from_repr(usize::from_be_bytes(raw.try_into()?)).context("invalid EventType")
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok((value as usize).to_be_bytes().to_vec())
	}
}

impl TuplePack for InnerEventTypeKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		pack_history_key(
			self.workflow_id,
			&self.location,
			w,
			tuple_depth,
			self.forgotten,
			INNER_EVENT_TYPE,
		)
	}
}

impl<'de> TupleUnpack<'de> for InnerEventTypeKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (workflow_id, location, forgotten)) =
			unpack_history_key(input, tuple_depth, INNER_EVENT_TYPE, "INNER_EVENT_TYPE")?;

		Ok((
			input,
			InnerEventTypeKey {
				workflow_id,
				location,
				forgotten,
			},
		))
	}
}

pub struct TagKey {
	workflow_id: Id,
	location: Location,
	forgotten: bool,
	pub k: String,
	pub v: String,
}

impl TagKey {
	pub fn new(workflow_id: Id, location: Location, k: String, v: String) -> Self {
		TagKey {
			workflow_id,
			location,
			forgotten: false,
			k,
			v,
		}
	}
}

impl FormalKey for TagKey {
	type Value = ();

	fn deserialize(&self, _raw: &[u8]) -> Result<Self::Value> {
		Ok(())
	}

	fn serialize(&self, _value: Self::Value) -> Result<Vec<u8>> {
		Ok(Vec::new())
	}
}

impl TuplePack for TagKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let mut offset = VersionstampOffset::None { size: 0 };

		offset += pack_history_key(
			self.workflow_id,
			&self.location,
			w,
			tuple_depth,
			self.forgotten,
			TAG,
		)?;

		let t = (&self.k, &self.v);
		offset += t.pack(w, tuple_depth)?;

		Ok(offset)
	}
}

impl<'de> TupleUnpack<'de> for TagKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (workflow_id, location, forgotten)) =
			unpack_history_key(input, tuple_depth, TAG, "TAG")?;

		let (input, (k, v)) = <(String, String)>::unpack(input, tuple_depth)?;

		Ok((
			input,
			TagKey {
				workflow_id,
				location,
				forgotten,
				k,
				v,
			},
		))
	}
}

fn pack_history_key<W: std::io::Write>(
	workflow_id: Id,
	location: &Location,
	w: &mut W,
	tuple_depth: TupleDepth,
	forgotten: bool,
	variant: usize,
) -> std::io::Result<VersionstampOffset> {
	let mut offset = VersionstampOffset::None { size: 0 };

	let t = (
		WORKFLOW,
		DATA,
		workflow_id,
		HISTORY,
		if forgotten { FORGOTTEN } else { ACTIVE },
	);
	offset += t.pack(w, tuple_depth)?;

	for coord in &**location {
		offset += coord.pack(w, tuple_depth)?;
	}

	let t = (DATA, variant);
	offset += t.pack(w, tuple_depth)?;

	Ok(offset)
}

fn unpack_history_key<'de>(
	input: &'de [u8],
	tuple_depth: TupleDepth,
	variant: usize,
	variant_str: &str,
) -> PackResult<(&'de [u8], (Id, Location, bool))> {
	let (mut input, (_, _, workflow_id, data, history_variant)) =
		<(usize, usize, Id, usize, usize)>::unpack(input, tuple_depth)?;
	if data != HISTORY {
		return Err(PackError::Message("expected HISTORY data".into()));
	}

	let mut coords = Vec::new();

	loop {
		let Ok((input2, coord)) = Coordinate::unpack(input, tuple_depth) else {
			break;
		};

		coords.push(coord);
		input = input2;
	}

	let (input, (_, data)) = <(usize, usize)>::unpack(input, tuple_depth)?;

	if data != variant {
		return Err(PackError::Message(
			format!("expected {variant_str} data").into(),
		));
	}

	Ok((
		input,
		(
			workflow_id,
			Location::from_iter(coords),
			history_variant == FORGOTTEN,
		),
	))
}

pub mod insert {
	use anyhow::Result;
	use rivet_util::Id;
	use udb_util::{FormalChunkedKey, FormalKey};
	use universaldb as udb;

	use super::super::super::value_to_str;
	use crate::{
		error::{WorkflowError, WorkflowResult},
		history::{
			event::{EventType, SleepState},
			location::Location,
		},
	};

	pub fn common(
		subspace: &udb::tuple::Subspace,
		tx: &udb::RetryableTransaction,
		workflow_id: Id,
		location: &Location,
		event_type: EventType,
		version: usize,
		create_ts: i64,
	) -> Result<()> {
		let event_type_key = super::EventTypeKey::new(workflow_id, location.clone());
		tx.set(
			&subspace.pack(&event_type_key),
			&event_type_key.serialize(event_type)?,
		);

		let version_key = super::VersionKey::new(workflow_id, location.clone());
		tx.set(
			&subspace.pack(&version_key),
			&version_key.serialize(version)?,
		);

		let create_ts_key = super::CreateTsKey::new(workflow_id, location.clone());
		tx.set(
			&subspace.pack(&create_ts_key),
			&create_ts_key.serialize(create_ts)?,
		);

		Ok(())
	}

	pub fn signal_event(
		subspace: &udb::tuple::Subspace,
		tx: &udb::RetryableTransaction,
		workflow_id: Id,
		location: &Location,
		version: usize,
		create_ts: i64,
		signal_id: Id,
		signal_name: &str,
		body: &serde_json::value::RawValue,
	) -> Result<()> {
		common(
			subspace,
			tx,
			workflow_id,
			location,
			EventType::Signal,
			version,
			create_ts,
		)?;

		let signal_id_key = super::SignalIdKey::new(workflow_id, location.clone());
		tx.set(
			&subspace.pack(&signal_id_key),
			&signal_id_key.serialize(signal_id)?,
		);

		let signal_name_key = super::NameKey::new(workflow_id, location.clone());
		tx.set(
			&subspace.pack(&signal_name_key),
			&signal_name_key.serialize(signal_name.to_string())?,
		);

		let signal_body_key = super::InputKey::new(workflow_id, location.clone());

		// Write signal body
		for (i, chunk) in signal_body_key
			.split_ref(&body)
			.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?
			.into_iter()
			.enumerate()
		{
			let chunk_key = signal_body_key.chunk(i);

			tx.set(&subspace.pack(&chunk_key), &chunk);
		}

		Ok(())
	}

	pub fn signal_send_event(
		subspace: &udb::tuple::Subspace,
		tx: &udb::RetryableTransaction,
		workflow_id: Id,
		location: &Location,
		version: usize,
		create_ts: i64,
		signal_id: Id,
		signal_name: &str,
		body: &serde_json::value::RawValue,
		to_workflow_id: Id,
	) -> Result<()> {
		common(
			subspace,
			tx,
			workflow_id,
			location,
			EventType::SignalSend,
			version,
			create_ts,
		)?;

		let signal_id_key = super::SignalIdKey::new(workflow_id, location.clone());
		tx.set(
			&subspace.pack(&signal_id_key),
			&signal_id_key.serialize(signal_id)?,
		);

		let signal_name_key = super::NameKey::new(workflow_id, location.clone());
		tx.set(
			&subspace.pack(&signal_name_key),
			&signal_name_key.serialize(signal_name.to_string())?,
		);

		let signal_body_key = super::InputKey::new(workflow_id, location.clone());

		// Write signal body
		for (i, chunk) in signal_body_key
			.split_ref(&body)
			.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?
			.into_iter()
			.enumerate()
		{
			let chunk_key = signal_body_key.chunk(i);

			tx.set(&subspace.pack(&chunk_key), &chunk);
		}

		let to_workflow_id_key = super::SubWorkflowIdKey::new(workflow_id, location.clone());
		tx.set(
			&subspace.pack(&to_workflow_id_key),
			&to_workflow_id_key.serialize(to_workflow_id)?,
		);

		Ok(())
	}

	pub fn sub_workflow_event(
		subspace: &udb::tuple::Subspace,
		tx: &udb::RetryableTransaction,
		workflow_id: Id,
		location: &Location,
		version: usize,
		create_ts: i64,
		sub_workflow_id: Id,
		sub_workflow_name: &str,
		tags: Option<&serde_json::Value>,
		input: &serde_json::value::RawValue,
	) -> Result<()> {
		common(
			subspace,
			tx,
			workflow_id,
			location,
			EventType::SubWorkflow,
			version,
			create_ts,
		)?;

		let sub_workflow_id_key = super::SubWorkflowIdKey::new(workflow_id, location.clone());
		tx.set(
			&subspace.pack(&sub_workflow_id_key),
			&sub_workflow_id_key.serialize(sub_workflow_id)?,
		);

		let sub_workflow_name_key = super::NameKey::new(workflow_id, location.clone());
		tx.set(
			&subspace.pack(&sub_workflow_name_key),
			&sub_workflow_name_key.serialize(sub_workflow_name.to_string())?,
		);

		// Write tags
		let tags = tags
			.map(|x| {
				x.as_object()
					.ok_or_else(|| WorkflowError::InvalidTags("must be an object".to_string()))
			})
			.transpose()
			.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?
			.into_iter()
			.flatten()
			.map(|(k, v)| Ok((k.clone(), value_to_str(v)?)))
			.collect::<WorkflowResult<Vec<_>>>()
			.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;

		for (k, v) in &tags {
			// Write tag key
			let tag_key = super::TagKey::new(workflow_id, location.clone(), k.clone(), v.clone());
			tx.set(
				&subspace.pack(&tag_key),
				&tag_key
					.serialize(())
					.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?,
			);
		}

		let input_key = super::InputKey::new(workflow_id, location.clone());

		// Write input
		for (i, chunk) in input_key
			.split_ref(&input)
			.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?
			.into_iter()
			.enumerate()
		{
			let chunk_key = input_key.chunk(i);

			tx.set(&subspace.pack(&chunk_key), &chunk);
		}

		Ok(())
	}

	pub fn activity_event(
		subspace: &udb::tuple::Subspace,
		tx: &udb::RetryableTransaction,
		workflow_id: Id,
		location: &Location,
		version: usize,
		create_ts: i64,
		activity_name: &str,
		input_hash: &[u8],
		input: &serde_json::value::RawValue,
		res: std::result::Result<&serde_json::value::RawValue, &str>,
	) -> Result<()> {
		common(
			subspace,
			tx,
			workflow_id,
			location,
			EventType::Activity,
			version,
			create_ts,
		)?;

		let activity_name_key = super::NameKey::new(workflow_id, location.clone());
		tx.set(
			&subspace.pack(&activity_name_key),
			&activity_name_key.serialize(activity_name.to_string())?,
		);

		let input_hash_key = super::InputHashKey::new(workflow_id, location.clone());
		tx.set(
			&subspace.pack(&input_hash_key),
			&input_hash_key.serialize(input_hash.to_vec())?,
		);

		let input_key = super::InputKey::new(workflow_id, location.clone());

		// Write input
		for (i, chunk) in input_key
			.split_ref(&input)
			.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?
			.into_iter()
			.enumerate()
		{
			let chunk_key = input_key.chunk(i);

			tx.set(&subspace.pack(&chunk_key), &chunk);
		}

		match res {
			Ok(output) => {
				let output_key = super::OutputKey::new(workflow_id, location.clone());

				// Write output
				for (i, chunk) in output_key
					.split_ref(&output)
					.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?
					.into_iter()
					.enumerate()
				{
					let chunk_key = output_key.chunk(i);

					tx.set(&subspace.pack(&chunk_key), &chunk);
				}
			}
			Err(err) => {
				let error_key = super::ErrorKey::new(
					workflow_id,
					location.clone(),
					rivet_util::timestamp::now(),
					err.to_string(),
				);
				tx.set(&subspace.pack(&error_key), &error_key.serialize(())?);
			}
		}

		Ok(())
	}

	pub fn message_send_event(
		subspace: &udb::tuple::Subspace,
		tx: &udb::RetryableTransaction,
		workflow_id: Id,
		location: &Location,
		version: usize,
		create_ts: i64,
		tags: &serde_json::Value,
		message_name: &str,
		body: &serde_json::value::RawValue,
	) -> Result<()> {
		common(
			subspace,
			tx,
			workflow_id,
			location,
			EventType::MessageSend,
			version,
			create_ts,
		)?;

		// Write tags
		let tags = tags
			.as_object()
			.ok_or_else(|| WorkflowError::InvalidTags("must be an object".to_string()))
			.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?
			.into_iter()
			.map(|(k, v)| Ok((k.clone(), value_to_str(v)?)))
			.collect::<WorkflowResult<Vec<_>>>()
			.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?;

		for (k, v) in &tags {
			// Write tag key
			let tag_key = super::TagKey::new(workflow_id, location.clone(), k.clone(), v.clone());
			tx.set(
				&subspace.pack(&tag_key),
				&tag_key
					.serialize(())
					.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?,
			);
		}

		let message_name_key = super::NameKey::new(workflow_id, location.clone());
		tx.set(
			&subspace.pack(&message_name_key),
			&message_name_key.serialize(message_name.to_string())?,
		);

		let body_key = super::InputKey::new(workflow_id, location.clone());

		// Write body
		for (i, chunk) in body_key
			.split_ref(&body)
			.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?
			.into_iter()
			.enumerate()
		{
			let chunk_key = body_key.chunk(i);

			tx.set(&subspace.pack(&chunk_key), &chunk);
		}

		Ok(())
	}

	pub fn loop_event(
		subspace: &udb::tuple::Subspace,
		tx: &udb::RetryableTransaction,
		workflow_id: Id,
		location: &Location,
		version: usize,
		create_ts: i64,
		iteration: usize,
		state: &serde_json::value::RawValue,
		output: Option<&serde_json::value::RawValue>,
	) -> Result<()> {
		common(
			subspace,
			tx,
			workflow_id,
			location,
			EventType::Loop,
			version,
			create_ts,
		)?;

		let iteration_key = super::IterationKey::new(workflow_id, location.clone());
		tx.set(
			&subspace.pack(&iteration_key),
			&iteration_key.serialize(iteration)?,
		);

		let state_key = super::InputKey::new(workflow_id, location.clone());

		// Write state
		for (i, chunk) in state_key
			.split_ref(&state)
			.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?
			.into_iter()
			.enumerate()
		{
			let chunk_key = state_key.chunk(i);

			tx.set(&subspace.pack(&chunk_key), &chunk);
		}

		if let Some(output) = output {
			let output_key = super::OutputKey::new(workflow_id, location.clone());

			// Write output
			for (i, chunk) in output_key
				.split_ref(&output)
				.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?
				.into_iter()
				.enumerate()
			{
				let chunk_key = output_key.chunk(i);

				tx.set(&subspace.pack(&chunk_key), &chunk);
			}
		}

		Ok(())
	}

	pub fn update_loop_event(
		subspace: &udb::tuple::Subspace,
		tx: &udb::RetryableTransaction,
		workflow_id: Id,
		location: &Location,
		iteration: usize,
		state: &serde_json::value::RawValue,
		output: Option<&serde_json::value::RawValue>,
	) -> Result<()> {
		let iteration_key = super::IterationKey::new(workflow_id, location.clone());
		tx.set(
			&subspace.pack(&iteration_key),
			&iteration_key.serialize(iteration)?,
		);

		let state_key = super::InputKey::new(workflow_id, location.clone());

		// Write state
		for (i, chunk) in state_key
			.split_ref(&state)
			.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?
			.into_iter()
			.enumerate()
		{
			let chunk_key = state_key.chunk(i);

			tx.set(&subspace.pack(&chunk_key), &chunk);
		}

		if let Some(output) = output {
			let output_key = super::OutputKey::new(workflow_id, location.clone());

			// Write output
			for (i, chunk) in output_key
				.split_ref(&output)
				.map_err(|x| udb::FdbBindingError::CustomError(x.into()))?
				.into_iter()
				.enumerate()
			{
				let chunk_key = output_key.chunk(i);

				tx.set(&subspace.pack(&chunk_key), &chunk);
			}
		}

		Ok(())
	}

	pub fn sleep_event(
		subspace: &udb::tuple::Subspace,
		tx: &udb::RetryableTransaction,
		workflow_id: Id,
		location: &Location,
		version: usize,
		create_ts: i64,
		deadline_ts: i64,
		sleep_state: SleepState,
	) -> Result<()> {
		common(
			subspace,
			tx,
			workflow_id,
			location,
			EventType::Sleep,
			version,
			create_ts,
		)?;

		let deadline_ts_key = super::DeadlineTsKey::new(workflow_id, location.clone());
		tx.set(
			&subspace.pack(&deadline_ts_key),
			&deadline_ts_key.serialize(deadline_ts)?,
		);

		let sleep_state_key = super::SleepStateKey::new(workflow_id, location.clone());
		tx.set(
			&subspace.pack(&sleep_state_key),
			&sleep_state_key.serialize(sleep_state)?,
		);

		Ok(())
	}

	pub fn update_sleep_event(
		subspace: &udb::tuple::Subspace,
		tx: &udb::RetryableTransaction,
		workflow_id: Id,
		location: &Location,
		sleep_state: SleepState,
	) -> Result<()> {
		let sleep_state_key = super::SleepStateKey::new(workflow_id, location.clone());
		tx.set(
			&subspace.pack(&sleep_state_key),
			&sleep_state_key.serialize(sleep_state)?,
		);

		Ok(())
	}

	pub fn branch_event(
		subspace: &udb::tuple::Subspace,
		tx: &udb::RetryableTransaction,
		workflow_id: Id,
		location: &Location,
		version: usize,
		create_ts: i64,
	) -> Result<()> {
		common(
			subspace,
			tx,
			workflow_id,
			location,
			EventType::Branch,
			version,
			create_ts,
		)?;

		Ok(())
	}

	pub fn removed_event(
		subspace: &udb::tuple::Subspace,
		tx: &udb::RetryableTransaction,
		workflow_id: Id,
		location: &Location,
		version: usize,
		create_ts: i64,
		inner_event_type: EventType,
		inner_event_name: Option<&str>,
	) -> Result<()> {
		common(
			subspace,
			tx,
			workflow_id,
			location,
			EventType::MessageSend,
			version,
			create_ts,
		)?;

		let inner_event_type_key = super::InnerEventTypeKey::new(workflow_id, location.clone());
		tx.set(
			&subspace.pack(&inner_event_type_key),
			&inner_event_type_key.serialize(inner_event_type)?,
		);

		if let Some(inner_event_name) = inner_event_name {
			let inner_event_name_key = super::NameKey::new(workflow_id, location.clone());
			tx.set(
				&subspace.pack(&inner_event_name_key),
				&inner_event_name_key.serialize(inner_event_name.to_string())?,
			);
		}

		Ok(())
	}

	pub fn version_check_event(
		subspace: &udb::tuple::Subspace,
		tx: &udb::RetryableTransaction,
		workflow_id: Id,
		location: &Location,
		version: usize,
		create_ts: i64,
	) -> Result<()> {
		common(
			subspace,
			tx,
			workflow_id,
			location,
			EventType::VersionCheck,
			version,
			create_ts,
		)?;

		Ok(())
	}
}

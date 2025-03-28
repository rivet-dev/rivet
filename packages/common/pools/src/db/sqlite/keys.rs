use fdb_util::prelude::*;
use std::{result::Result::Ok, sync::Arc};

pub struct DbDataKey {
	db_name_segment: Arc<Vec<u8>>,
}

impl DbDataKey {
	pub fn new(db_name_segment: Arc<Vec<u8>>) -> Self {
		DbDataKey { db_name_segment }
	}
}

impl TuplePack for DbDataKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (DBS, &*self.db_name_segment, DATA);
		t.pack(w, tuple_depth)
	}
}

/// Uncompressed data.
pub struct DbDataChunkKey {
	#[allow(dead_code)]
	pub db_name_segment: Arc<Vec<u8>>,
	pub chunk: usize,
}

impl<'de> TupleUnpack<'de> for DbDataChunkKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, db_name_segment, _, chunk)) =
			<(usize, Vec<u8>, usize, usize)>::unpack(input, tuple_depth)?;
		let v = DbDataChunkKey {
			db_name_segment: Arc::new(db_name_segment),
			chunk,
		};

		Ok((input, v))
	}
}

pub struct CompressedDbDataKey {
	db_name_segment: Arc<Vec<u8>>,
}

impl CompressedDbDataKey {
	pub fn new(db_name_segment: Arc<Vec<u8>>) -> Self {
		CompressedDbDataKey { db_name_segment }
	}
}

impl TuplePack for CompressedDbDataKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (DBS, &*self.db_name_segment, COMPRESSED_DATA);
		t.pack(w, tuple_depth)
	}
}

pub struct CompressedDbDataChunkKey {
	pub db_name_segment: Arc<Vec<u8>>,
	pub chunk: usize,
}

impl TuplePack for CompressedDbDataChunkKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (DBS, &*self.db_name_segment, COMPRESSED_DATA, self.chunk);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for CompressedDbDataChunkKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, db_name_segment, _, chunk)) =
			<(usize, Vec<u8>, usize, usize)>::unpack(input, tuple_depth)?;
		let v = CompressedDbDataChunkKey {
			db_name_segment: Arc::new(db_name_segment),
			chunk,
		};

		Ok((input, v))
	}
}

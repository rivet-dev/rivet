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

//impl FormalChunkedKey for SqliteDbKey {
//	type ChunkKey = SqliteDbKey;
//	type Value = Vec<u8>;
//
//	fn chunk(&self, chunk: usize) -> Self::ChunkKey {
//		SqliteDbKey {
//			workflow_id: self.workflow_id,
//			chunk,
//		}
//	}
//
//	fn combine(&self, chunks: Vec<FdbValue>) -> Result<Self::Value> {
//		TODO
//	}
//
//	fn split(&self, value: Self::Value) -> Result<Vec<Vec<u8>>> {
//		self.split_ref(value.as_ref())
//	}
//}

impl TuplePack for DbDataKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = ("dbs", &*self.db_name_segment, "data");
		t.pack(w, tuple_depth)
	}
}

pub struct DbDataChunkKey {
	pub db_name_segment: Arc<Vec<u8>>,
	pub chunk: usize,
}

impl TuplePack for DbDataChunkKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (DBS, &*self.db_name_segment, DATA, self.chunk);
		t.pack(w, tuple_depth)
	}
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

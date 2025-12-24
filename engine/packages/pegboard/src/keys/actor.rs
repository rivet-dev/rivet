use anyhow::Result;
use gas::prelude::*;
use rivet_runner_protocol as protocol;
use universaldb::prelude::*;

#[derive(Debug)]
pub struct CreateTsKey {
	actor_id: Id,
}

impl CreateTsKey {
	pub fn new(actor_id: Id) -> Self {
		CreateTsKey { actor_id }
	}
}

impl FormalKey for CreateTsKey {
	// Timestamp.
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
		let t = (ACTOR, DATA, self.actor_id, CREATE_TS);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for CreateTsKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, actor_id, _)) = <(usize, usize, Id, usize)>::unpack(input, tuple_depth)?;
		let v = CreateTsKey { actor_id };

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct WorkflowIdKey {
	actor_id: Id,
}

impl WorkflowIdKey {
	pub fn new(actor_id: Id) -> Self {
		WorkflowIdKey { actor_id }
	}
}

impl FormalKey for WorkflowIdKey {
	type Value = Id;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(Id::from_slice(raw)?)
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.as_bytes().to_vec())
	}
}

impl TuplePack for WorkflowIdKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (ACTOR, DATA, self.actor_id, WORKFLOW_ID);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for WorkflowIdKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, actor_id, _)) = <(usize, usize, Id, usize)>::unpack(input, tuple_depth)?;

		let v = WorkflowIdKey { actor_id };

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct RunnerIdKey {
	actor_id: Id,
}

impl RunnerIdKey {
	pub fn new(actor_id: Id) -> Self {
		RunnerIdKey { actor_id }
	}
}

impl FormalKey for RunnerIdKey {
	type Value = Id;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(Id::from_slice(raw)?)
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.as_bytes())
	}
}

impl TuplePack for RunnerIdKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (ACTOR, DATA, self.actor_id, RUNNER_ID);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for RunnerIdKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, actor_id, _)) = <(usize, usize, Id, usize)>::unpack(input, tuple_depth)?;

		let v = RunnerIdKey { actor_id };

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct ConnectableKey {
	actor_id: Id,
}

impl ConnectableKey {
	pub fn new(actor_id: Id) -> Self {
		ConnectableKey { actor_id }
	}
}

impl FormalKey for ConnectableKey {
	type Value = ();

	fn deserialize(&self, _raw: &[u8]) -> Result<Self::Value> {
		Ok(())
	}

	fn serialize(&self, _value: Self::Value) -> Result<Vec<u8>> {
		Ok(Vec::new())
	}
}

impl TuplePack for ConnectableKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (ACTOR, DATA, self.actor_id, CONNECTABLE);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for ConnectableKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, actor_id, _)) = <(usize, usize, Id, usize)>::unpack(input, tuple_depth)?;

		let v = ConnectableKey { actor_id };

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct SleepTsKey {
	actor_id: Id,
}

impl SleepTsKey {
	pub fn new(actor_id: Id) -> Self {
		SleepTsKey { actor_id }
	}
}

impl FormalKey for SleepTsKey {
	// Timestamp.
	type Value = i64;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(i64::from_be_bytes(raw.try_into()?))
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.to_be_bytes().to_vec())
	}
}

impl TuplePack for SleepTsKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (ACTOR, DATA, self.actor_id, SLEEP_TS);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for SleepTsKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, actor_id, _)) = <(usize, usize, Id, usize)>::unpack(input, tuple_depth)?;

		let v = SleepTsKey { actor_id };

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct DestroyTsKey {
	actor_id: Id,
}

impl DestroyTsKey {
	pub fn new(actor_id: Id) -> Self {
		DestroyTsKey { actor_id }
	}
}

impl FormalKey for DestroyTsKey {
	// Timestamp.
	type Value = i64;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(i64::from_be_bytes(raw.try_into()?))
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.to_be_bytes().to_vec())
	}
}

impl TuplePack for DestroyTsKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (ACTOR, DATA, self.actor_id, DESTROY_TS);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for DestroyTsKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, actor_id, _)) = <(usize, usize, Id, usize)>::unpack(input, tuple_depth)?;

		let v = DestroyTsKey { actor_id };

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct NamespaceIdKey {
	actor_id: Id,
}

impl NamespaceIdKey {
	pub fn new(actor_id: Id) -> Self {
		NamespaceIdKey { actor_id }
	}
}

impl FormalKey for NamespaceIdKey {
	type Value = Id;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(Id::from_slice(raw)?)
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.as_bytes())
	}
}

impl TuplePack for NamespaceIdKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (ACTOR, DATA, self.actor_id, NAMESPACE_ID);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for NamespaceIdKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, actor_id, _)) = <(usize, usize, Id, usize)>::unpack(input, tuple_depth)?;

		let v = NamespaceIdKey { actor_id };

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct RunnerNameSelectorKey {
	actor_id: Id,
}

impl RunnerNameSelectorKey {
	pub fn new(actor_id: Id) -> Self {
		RunnerNameSelectorKey { actor_id }
	}
}

impl FormalKey for RunnerNameSelectorKey {
	type Value = String;

	fn deserialize(&self, raw: &[u8]) -> Result<Self::Value> {
		Ok(String::from_utf8(raw.to_vec())?)
	}

	fn serialize(&self, value: Self::Value) -> Result<Vec<u8>> {
		Ok(value.into_bytes())
	}
}

impl TuplePack for RunnerNameSelectorKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (ACTOR, DATA, self.actor_id, RUNNER_NAME_SELECTOR);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for RunnerNameSelectorKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, actor_id, _)) = <(usize, usize, Id, usize)>::unpack(input, tuple_depth)?;

		let v = RunnerNameSelectorKey { actor_id };

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct HibernatingRequestKey {
	actor_id: Id,
	last_ping_ts: i64,
	pub gateway_id: protocol::GatewayId,
	pub request_id: protocol::RequestId,
}

impl HibernatingRequestKey {
	pub fn new(
		actor_id: Id,
		last_ping_ts: i64,
		gateway_id: protocol::GatewayId,
		request_id: protocol::RequestId,
	) -> Self {
		HibernatingRequestKey {
			actor_id,
			last_ping_ts,
			gateway_id,
			request_id,
		}
	}

	pub fn subspace_with_ts(actor_id: Id, last_ping_ts: i64) -> HibernatingRequestSubspaceKey {
		HibernatingRequestSubspaceKey::new_with_ts(actor_id, last_ping_ts)
	}

	pub fn subspace(actor_id: Id) -> HibernatingRequestSubspaceKey {
		HibernatingRequestSubspaceKey::new(actor_id)
	}
}

impl FormalKey for HibernatingRequestKey {
	type Value = ();

	fn deserialize(&self, _raw: &[u8]) -> Result<Self::Value> {
		Ok(())
	}

	fn serialize(&self, _value: Self::Value) -> Result<Vec<u8>> {
		Ok(Vec::new())
	}
}

impl TuplePack for HibernatingRequestKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let t = (
			ACTOR,
			HIBERNATING_REQUEST,
			self.actor_id,
			self.last_ping_ts,
			&self.gateway_id[..],
			&self.request_id[..],
		);
		t.pack(w, tuple_depth)
	}
}

impl<'de> TupleUnpack<'de> for HibernatingRequestKey {
	fn unpack(input: &[u8], tuple_depth: TupleDepth) -> PackResult<(&[u8], Self)> {
		let (input, (_, _, actor_id, last_ping_ts, gateway_id_bytes, request_id_bytes)) =
			<(usize, usize, Id, i64, Vec<u8>, Vec<u8>)>::unpack(input, tuple_depth)?;

		let gateway_id = gateway_id_bytes
			.as_slice()
			.try_into()
			.expect("invalid gateway_id length");

		let request_id = request_id_bytes
			.as_slice()
			.try_into()
			.expect("invalid request_id length");

		let v = HibernatingRequestKey {
			actor_id,
			last_ping_ts,
			gateway_id,
			request_id,
		};

		Ok((input, v))
	}
}

#[derive(Debug)]
pub struct HibernatingRequestSubspaceKey {
	actor_id: Id,
	last_ping_ts: Option<i64>,
}

impl HibernatingRequestSubspaceKey {
	pub fn new(actor_id: Id) -> Self {
		HibernatingRequestSubspaceKey {
			actor_id,
			last_ping_ts: None,
		}
	}

	pub fn new_with_ts(actor_id: Id, last_ping_ts: i64) -> Self {
		HibernatingRequestSubspaceKey {
			actor_id,
			last_ping_ts: Some(last_ping_ts),
		}
	}
}

impl TuplePack for HibernatingRequestSubspaceKey {
	fn pack<W: std::io::Write>(
		&self,
		w: &mut W,
		tuple_depth: TupleDepth,
	) -> std::io::Result<VersionstampOffset> {
		let mut offset = VersionstampOffset::None { size: 0 };

		let t = (ACTOR, HIBERNATING_REQUEST, self.actor_id);
		offset += t.pack(w, tuple_depth)?;

		if let Some(last_ping_ts) = self.last_ping_ts {
			offset += last_ping_ts.pack(w, tuple_depth)?;
		}

		Ok(offset)
	}
}

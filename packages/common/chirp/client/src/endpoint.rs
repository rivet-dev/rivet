use std::fmt::Debug;

pub fn subject(name: &str) -> String {
	format!("chirp.rpc.{}", name)
}

/// Holds data returned from an RPC call.
#[derive(Debug)]
pub struct RpcResponse<Res>
where
	Res: Debug,
{
	pub(crate) body: Res,
}

impl<Res> RpcResponse<Res>
where
	Res: Debug,
{
	pub fn body(&self) -> &Res {
		&self.body
	}
}

impl<Res> std::ops::Deref for RpcResponse<Res>
where
	Res: Debug,
{
	type Target = Res;

	fn deref(&self) -> &Self::Target {
		&self.body
	}
}
